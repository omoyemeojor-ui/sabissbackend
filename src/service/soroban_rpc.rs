use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signer as _, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use stellar_strkey::{Strkey, ed25519};
use stellar_xdr::curr::{
    AccountId, ContractId, DecoratedSignature, Hash, Int128Parts, InvokeContractArgs,
    InvokeHostFunctionOp, Limits, Memo, MuxedAccount, Operation, OperationBody, Preconditions,
    PublicKey, ReadXdr, ScAddress, ScBytes, ScSymbol, ScVal, ScVec, SequenceNumber,
    Signature, SignatureHint, SorobanAuthorizationEntry, SorobanTransactionData, Transaction,
    TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256, WriteXdr,
};

use crate::config::environment::Environment;

const BASE_TRANSACTION_FEE: u32 = 100;
const TRANSACTION_POLL_ATTEMPTS: usize = 20;
const TRANSACTION_POLL_DELAY_MS: u64 = 500;

pub struct SorobanRpc {
    client: Client,
    rpc_urls: Vec<String>,
    horizon_urls: Vec<String>,
    network_passphrase: String,
    simulation_source_account: String,
}

pub struct InvokeResponse {
    pub tx_hash: String,
    pub value: String,
}

#[derive(Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: &'static str,
    id: u32,
    method: &'static str,
    params: T,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i32,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

#[derive(Serialize)]
struct SimulateTransactionParams {
    transaction: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimulateTransactionResult {
    #[serde(default)]
    transaction_data: Option<String>,
    #[serde(default)]
    min_resource_fee: Option<String>,
    #[serde(default)]
    results: Vec<SimulatedInvocationResult>,
}

#[derive(Deserialize)]
struct SimulatedInvocationResult {
    xdr: String,
    #[serde(default)]
    auth: Vec<String>,
}

#[derive(Serialize)]
struct SendTransactionParams {
    transaction: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendTransactionResult {
    status: String,
    hash: Option<String>,
    #[serde(default)]
    error_result_xdr: Option<String>,
}

#[derive(Serialize)]
struct GetTransactionParams {
    hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTransactionResult {
    status: String,
    #[allow(dead_code)]
    result_xdr: Option<String>,
}

#[derive(Deserialize)]
struct HorizonAccountResponse {
    sequence: String,
}

struct SimulatedTransaction {
    value: String,
    min_resource_fee: u32,
    auth: Vec<SorobanAuthorizationEntry>,
    transaction_data: SorobanTransactionData,
}

impl SorobanRpc {
    pub fn new(env: &Environment) -> Self {
        let rpc_urls = env
            .rpc_candidates()
            .into_iter()
            .map(normalize_rpc_url)
            .collect();
        let horizon_urls = env
            .horizon_candidates()
            .into_iter()
            .map(|url| url.trim_end_matches('/').to_owned())
            .collect();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            rpc_urls,
            horizon_urls,
            network_passphrase: network_passphrase(&env.network).to_owned(),
            simulation_source_account: env.stellar_aa_sponsor_address.clone(),
        }
    }

    pub async fn invoke(
        &self,
        contract_id: &str,
        method: &str,
        args: &[(&str, &str)],
        source_account: &str,
        secret_key: &str,
    ) -> Result<InvokeResponse> {
        let source_account = resolve_source_account(source_account, secret_key)?;
        let sequence = self
            .get_account_sequence(&source_account)
            .await?
            .parse::<i64>()
            .with_context(|| format!("invalid account sequence for `{source_account}`"))?;

        let unsigned_tx = build_invoke_transaction(
            &source_account,
            sequence + 1,
            BASE_TRANSACTION_FEE,
            contract_id,
            method,
            args,
            Vec::new(),
            TransactionExt::V0,
        )?;

        let simulated = self.simulate_transaction(&unsigned_tx).await?;
        let fee = BASE_TRANSACTION_FEE
            .checked_add(simulated.min_resource_fee)
            .ok_or_else(|| anyhow!("computed Soroban fee overflowed"))?;
        let final_tx = build_invoke_transaction(
            &source_account,
            sequence + 1,
            fee,
            contract_id,
            method,
            args,
            simulated.auth,
            TransactionExt::V1(simulated.transaction_data),
        )?;
        let tx_hash = self
            .submit_and_wait(&sign_transaction(final_tx, secret_key, &self.network_passphrase)?)
            .await?;

        Ok(InvokeResponse {
            tx_hash,
            value: simulated.value,
        })
    }

    pub async fn simulate(
        &self,
        contract_id: &str,
        method: &str,
        args: &[(&str, &str)],
    ) -> Result<String> {
        let sequence = self
            .get_account_sequence(&self.simulation_source_account)
            .await?
            .parse::<i64>()
            .with_context(|| {
                format!(
                    "invalid account sequence for `{}`",
                    self.simulation_source_account
                )
            })?;
        let tx = build_invoke_transaction(
            &self.simulation_source_account,
            sequence + 1,
            BASE_TRANSACTION_FEE,
            contract_id,
            method,
            args,
            Vec::new(),
            TransactionExt::V0,
        )?;

        Ok(self.simulate_transaction(&tx).await?.value)
    }

    pub async fn get_account_sequence(&self, address: &str) -> Result<String> {
        let mut failures = Vec::new();

        for horizon_url in &self.horizon_urls {
            let url = format!("{horizon_url}/accounts/{address}");
            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let account: HorizonAccountResponse = response
                        .json()
                        .await
                        .context("failed to parse Horizon account response")?;
                    return Ok(account.sequence);
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    failures.push(format!("{url} returned {status}: {body}"));
                }
                Err(error) => failures.push(format!("{url} failed: {error}")),
            }
        }

        Err(anyhow!(
            "failed to fetch account sequence for `{address}`: {}",
            failures.join(" | ")
        ))
    }

    async fn simulate_transaction(&self, tx: &TransactionEnvelope) -> Result<SimulatedTransaction> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "simulateTransaction",
            params: SimulateTransactionParams {
                transaction: tx_to_base64(tx)?,
            },
        };
        let result: SimulateTransactionResult = self.send_request(&request).await?;
        let entry = result
            .results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("simulateTransaction returned no invocation results"))?;

        let value = scval_to_contract_output(&decode_scval(&entry.xdr)?)?;
        let min_resource_fee = result
            .min_resource_fee
            .as_deref()
            .unwrap_or("0")
            .parse::<u32>()
            .context("invalid minResourceFee from simulateTransaction")?;
        let transaction_data_b64 = result
            .transaction_data
            .ok_or_else(|| anyhow!("simulateTransaction returned no transactionData"))?;
        let transaction_data = decode_xdr::<SorobanTransactionData>(&transaction_data_b64)
            .context("failed to decode simulated transaction data")?;
        let auth = entry
            .auth
            .into_iter()
            .map(|value| {
                decode_xdr::<SorobanAuthorizationEntry>(&value)
                    .context("failed to decode simulated authorization entry")
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(SimulatedTransaction {
            value,
            min_resource_fee,
            auth,
            transaction_data,
        })
    }

    async fn submit_and_wait(&self, tx: &TransactionEnvelope) -> Result<String> {
        let tx_hash = hex::encode(tx.hash(network_id(&self.network_passphrase))?);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "sendTransaction",
            params: SendTransactionParams {
                transaction: tx_to_base64(tx)?,
            },
        };
        let response: SendTransactionResult = self.send_request(&request).await?;

        match response.status.as_str() {
            "PENDING" | "DUPLICATE" => {}
            "ERROR" => {
                return Err(anyhow!(
                    "sendTransaction failed: {}",
                    response
                        .error_result_xdr
                        .unwrap_or_else(|| "missing error result XDR".to_owned())
                ));
            }
            other => return Err(anyhow!("unexpected sendTransaction status `{other}`")),
        }

        let rpc_hash = response.hash.unwrap_or(tx_hash);
        for _ in 0..TRANSACTION_POLL_ATTEMPTS {
            let request = JsonRpcRequest {
                jsonrpc: "2.0",
                id: 1,
                method: "getTransaction",
                params: GetTransactionParams {
                    hash: rpc_hash.clone(),
                },
            };
            let result: GetTransactionResult = self.send_request(&request).await?;
            match result.status.as_str() {
                "SUCCESS" => return Ok(rpc_hash),
                "FAILED" => {
                    let detail = result
                        .result_xdr
                        .unwrap_or_else(|| "missing transaction result XDR".to_owned());
                    return Err(anyhow!("transaction failed on-chain: {detail}"));
                }
                "NOT_FOUND" => {
                    tokio::time::sleep(Duration::from_millis(TRANSACTION_POLL_DELAY_MS)).await;
                }
                other => return Err(anyhow!("unexpected getTransaction status `{other}`")),
            }
        }

        Err(anyhow!(
            "transaction `{rpc_hash}` was submitted but did not finalize before timeout"
        ))
    }

    async fn send_request<TReq, TRes>(&self, request: &TReq) -> Result<TRes>
    where
        TReq: Serialize,
        TRes: DeserializeOwned,
    {
        let mut failures = Vec::new();

        for rpc_url in &self.rpc_urls {
            let response = match self
                .client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(request)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    failures.push(format!("{rpc_url} failed: {error}"));
                    continue;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                failures.push(format!("{rpc_url} returned {status}: {body}"));
                continue;
            }

            let payload: JsonRpcResponse<TRes> = response
                .json()
                .await
                .context("failed to parse RPC response body")?;
            if let Some(error) = payload.error {
                return Err(anyhow!("RPC error {}: {}", error.code, error.message));
            }
            if let Some(result) = payload.result {
                return Ok(result);
            }

            return Err(anyhow!("RPC response did not contain a result"));
        }

        Err(anyhow!(
            "all Soroban RPC requests failed: {}",
            failures.join(" | ")
        ))
    }
}

fn build_invoke_transaction(
    source_account: &str,
    sequence: i64,
    fee: u32,
    contract_id: &str,
    method: &str,
    args: &[(&str, &str)],
    auth: Vec<SorobanAuthorizationEntry>,
    ext: TransactionExt,
) -> Result<TransactionEnvelope> {
    let source_account = stellar_account_to_muxed(source_account)?;
    let contract_address = stellar_address_to_sc_address(contract_id)?;
    let function_name = ScSymbol(method.as_bytes().to_vec().try_into()?);
    let args = args
        .iter()
        .map(|(name, value)| encode_contract_argument(method, name, value))
        .collect::<Result<Vec<_>>>()?;
    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: stellar_xdr::curr::HostFunction::InvokeContract(InvokeContractArgs {
                contract_address,
                function_name,
                args: args.try_into()?,
            }),
            auth: auth.try_into()?,
        }),
    };
    let tx = Transaction {
        source_account,
        fee,
        seq_num: SequenceNumber(sequence),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![op].try_into()?,
        ext,
    };

    Ok(TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: Vec::new().try_into()?,
    }))
}

fn sign_transaction(
    tx: TransactionEnvelope,
    secret_key: &str,
    network_passphrase: &str,
) -> Result<TransactionEnvelope> {
    let (signing_key, public_key) = signing_key_from_secret(secret_key)?;
    let payload = tx.hash(network_id(network_passphrase))?;
    let signature = signing_key.sign(&payload);
    let decorated = DecoratedSignature {
        hint: SignatureHint(public_key[28..32].try_into()?),
        signature: Signature(signature.to_bytes().to_vec().try_into()?),
    };

    match tx {
        TransactionEnvelope::Tx(mut envelope) => {
            envelope.signatures = vec![decorated].try_into()?;
            Ok(TransactionEnvelope::Tx(envelope))
        }
        _ => Err(anyhow!("unsupported transaction envelope variant")),
    }
}

fn resolve_source_account(source_account: &str, secret_key: &str) -> Result<String> {
    if is_secret_key(source_account) {
        return derive_public_key_from_secret(source_account);
    }
    if !source_account.trim().is_empty() && is_stellar_address(source_account) {
        return Ok(source_account.trim().to_ascii_uppercase());
    }
    derive_public_key_from_secret(secret_key)
}

fn derive_public_key_from_secret(secret_key: &str) -> Result<String> {
    let (_, public_key) = signing_key_from_secret(secret_key)?;
    Ok(Strkey::PublicKeyEd25519(ed25519::PublicKey(public_key))
        .to_string()
        .to_string())
}

fn signing_key_from_secret(secret_key: &str) -> Result<(SigningKey, [u8; 32])> {
    match Strkey::from_string(secret_key).context("invalid stellar secret key")? {
        Strkey::PrivateKeyEd25519(secret) => {
            let signing_key = SigningKey::from_bytes(&secret.0);
            let public_key = signing_key.verifying_key().to_bytes();
            Ok((signing_key, public_key))
        }
        _ => Err(anyhow!("expected an ed25519 stellar secret key")),
    }
}

fn encode_contract_argument(method: &str, name: &str, value: &str) -> Result<ScVal> {
    let name = name.trim();

    match (method, name) {
        ("create_wallet", "owner") => return bytes_scval(value),
        (_, "event-id" | "group-id" | "series-id" | "question-id" | "condition-id") => {
            return bytes_scval(value);
        }
        (_, "other-market" | "parent-collection-id" | "collection-id" | "position-id") => {
            return bytes_scval(value);
        }
        (_, "resolver" | "disputer" | "oracle" | "buyer" | "seller" | "user" | "provider") => {
            return address_scval(value);
        }
        (_, "recipient" | "collateral-token" | "to") => return address_scval(value),
        (_, "id") if is_stellar_address(value) => return address_scval(value),
        (_, "end-time") => return u64_scval(value),
        (_, "winning-outcome" | "outcome-index" | "price-bps" | "index-set") => {
            return u32_scval(value);
        }
        (_, "amount" | "yes-amount" | "no-amount" | "usdc-amount" | "token-amount") => {
            return i128_scval(value);
        }
        (_, "collateral-amount" | "pair-token-amount") => return i128_scval(value),
        (_, "neg-risk") => return bool_scval(value),
        (_, "partition") => return u32_vec_scval(value),
        _ => {}
    }

    if is_stellar_address(value) {
        return address_scval(value);
    }
    if is_bytes_literal(value) {
        return bytes_scval(value);
    }
    if let Ok(parsed) = parse_bool(value) {
        return Ok(ScVal::Bool(parsed));
    }
    if value.trim_start().starts_with('[') {
        return u32_vec_scval(value);
    }
    if value.parse::<i128>().is_ok() {
        return i128_scval(value);
    }

    Ok(ScVal::String(value.trim().as_bytes().to_vec().try_into()?))
}

fn address_scval(value: &str) -> Result<ScVal> {
    Ok(ScVal::Address(stellar_address_to_sc_address(value)?))
}

fn bytes_scval(value: &str) -> Result<ScVal> {
    Ok(ScVal::Bytes(ScBytes(hex_to_bytes(value)?.try_into()?)))
}

fn bool_scval(value: &str) -> Result<ScVal> {
    Ok(ScVal::Bool(parse_bool(value)?))
}

fn u32_scval(value: &str) -> Result<ScVal> {
    Ok(ScVal::U32(
        value
            .trim()
            .parse::<u32>()
            .with_context(|| format!("invalid u32 argument `{value}`"))?,
    ))
}

fn u64_scval(value: &str) -> Result<ScVal> {
    Ok(ScVal::U64(
        value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("invalid u64 argument `{value}`"))?,
    ))
}

fn i128_scval(value: &str) -> Result<ScVal> {
    let parsed = value
        .trim()
        .parse::<i128>()
        .with_context(|| format!("invalid i128 argument `{value}`"))?;
    let bytes = parsed.to_be_bytes();
    Ok(ScVal::I128(Int128Parts {
        hi: i64::from_be_bytes(bytes[..8].try_into()?),
        lo: u64::from_be_bytes(bytes[8..].try_into()?),
    }))
}

fn u32_vec_scval(value: &str) -> Result<ScVal> {
    let inner = value.trim();
    let inner = inner
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| anyhow!("invalid vector argument `{value}`"))?;
    let values = if inner.trim().is_empty() {
        Vec::new()
    } else {
        inner
            .split(',')
            .map(|entry| {
                entry
                    .trim()
                    .parse::<u32>()
                    .map(ScVal::U32)
                    .with_context(|| format!("invalid vector entry `{entry}` in `{value}`"))
            })
            .collect::<Result<Vec<_>>>()?
    };

    Ok(ScVal::Vec(Some(ScVec(values.try_into()?))))
}

fn stellar_account_to_muxed(address: &str) -> Result<MuxedAccount> {
    match Strkey::from_string(address).context("invalid stellar account address")? {
        Strkey::PublicKeyEd25519(public_key) => Ok(MuxedAccount::Ed25519(Uint256(public_key.0))),
        _ => Err(anyhow!("expected a stellar G-address for source account")),
    }
}

fn stellar_address_to_sc_address(address: &str) -> Result<ScAddress> {
    match Strkey::from_string(address.trim()).context("invalid stellar address")? {
        Strkey::PublicKeyEd25519(public_key) => Ok(ScAddress::Account(AccountId(
            PublicKey::PublicKeyTypeEd25519(Uint256(public_key.0)),
        ))),
        Strkey::Contract(contract) => Ok(ScAddress::Contract(ContractId(Hash(contract.0)))),
        _ => Err(anyhow!("unsupported stellar address `{address}`")),
    }
}

fn decode_scval(value: &str) -> Result<ScVal> {
    let decoded = BASE64
        .decode(value)
        .with_context(|| format!("failed to decode base64 XDR `{value}`"))?;
    Ok(ScVal::from_xdr(decoded, Limits::none())?)
}

fn decode_xdr<T: ReadXdr>(value: &str) -> Result<T> {
    let decoded = BASE64
        .decode(value)
        .with_context(|| format!("failed to decode base64 XDR `{value}`"))?;
    Ok(T::from_xdr(decoded, Limits::none())?)
}

fn tx_to_base64(tx: &TransactionEnvelope) -> Result<String> {
    let bytes = tx.to_xdr(Limits::none())?;
    Ok(BASE64.encode(bytes))
}

fn scval_to_contract_output(value: &ScVal) -> Result<String> {
    match value {
        ScVal::Void => Ok(String::new()),
        ScVal::Bool(value) => Ok(value.to_string()),
        ScVal::U32(value) => Ok(value.to_string()),
        ScVal::I32(value) => Ok(value.to_string()),
        ScVal::U64(value) => Ok(value.to_string()),
        ScVal::I64(value) => Ok(value.to_string()),
        ScVal::U128(value) => Ok(uint128_to_string(value)),
        ScVal::I128(value) => Ok(int128_to_string(value)),
        ScVal::Bytes(value) => Ok(bytes_to_contract_string(value.as_ref())),
        ScVal::String(value) => Ok(String::from_utf8_lossy(value.as_ref()).to_string()),
        ScVal::Symbol(value) => Ok(String::from_utf8_lossy(value.as_ref()).to_string()),
        ScVal::Address(value) => sc_address_to_string(value),
        ScVal::Vec(_) | ScVal::Map(_) => Ok(scval_to_json_value(value)?.to_string()),
        _ => Ok(scval_to_json_value(value)?.to_string()),
    }
}

fn scval_to_json_value(value: &ScVal) -> Result<Value> {
    Ok(match value {
        ScVal::Bool(value) => Value::Bool(*value),
        ScVal::Void => Value::Null,
        ScVal::U32(value) => Value::Number(Number::from(*value)),
        ScVal::I32(value) => Value::Number(Number::from(*value)),
        ScVal::U64(value) => Value::Number(Number::from(*value)),
        ScVal::I64(value) => Value::Number(Number::from(*value)),
        ScVal::U128(value) => Value::String(uint128_to_string(value)),
        ScVal::I128(value) => Value::String(int128_to_string(value)),
        ScVal::Bytes(value) => Value::String(bytes_to_contract_string(value.as_ref())),
        ScVal::String(value) => Value::String(String::from_utf8_lossy(value.as_ref()).to_string()),
        ScVal::Symbol(value) => Value::String(String::from_utf8_lossy(value.as_ref()).to_string()),
        ScVal::Address(value) => Value::String(sc_address_to_string(value)?),
        ScVal::Vec(Some(values)) => Value::Array(
            values
                .iter()
                .map(scval_to_json_value)
                .collect::<Result<Vec<_>>>()?,
        ),
        ScVal::Vec(None) => Value::Array(Vec::new()),
        ScVal::Map(Some(values)) => {
            let mut object = Map::new();
            for entry in values.iter() {
                object.insert(scval_to_json_key(&entry.key)?, scval_to_json_value(&entry.val)?);
            }
            Value::Object(object)
        }
        ScVal::Map(None) => Value::Object(Map::new()),
        other => Value::String(format!("{other:?}")),
    })
}

fn scval_to_json_key(value: &ScVal) -> Result<String> {
    match value {
        ScVal::String(value) => Ok(String::from_utf8_lossy(value.as_ref()).to_string()),
        ScVal::Symbol(value) => Ok(String::from_utf8_lossy(value.as_ref()).to_string()),
        _ => scval_to_contract_output(value),
    }
}

fn sc_address_to_string(value: &ScAddress) -> Result<String> {
    match value {
        ScAddress::Account(account_id) => match account_id.as_ref() {
            PublicKey::PublicKeyTypeEd25519(Uint256(bytes)) => Ok(
                Strkey::PublicKeyEd25519(ed25519::PublicKey(*bytes))
                    .to_string()
                    .to_string(),
            ),
        },
        ScAddress::Contract(contract_id) => {
            let Hash(bytes) = contract_id.0;
            Ok(Strkey::Contract(stellar_strkey::Contract(bytes))
                .to_string()
                .to_string())
        }
        _ => Err(anyhow!("unsupported SCAddress variant in contract output")),
    }
}

fn bytes_to_contract_string(bytes: &[u8]) -> String {
    if bytes.is_ascii() && bytes.iter().all(|byte| !byte.is_ascii_control()) {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        hex::encode(bytes)
    }
}

fn uint128_to_string(value: &stellar_xdr::curr::UInt128Parts) -> String {
    (((value.hi as u128) << 64) | value.lo as u128).to_string()
}

fn int128_to_string(value: &Int128Parts) -> String {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&value.hi.to_be_bytes());
    bytes[8..].copy_from_slice(&value.lo.to_be_bytes());
    i128::from_be_bytes(bytes).to_string()
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(anyhow!("invalid boolean argument `{value}`")),
    }
}

fn is_secret_key(value: &str) -> bool {
    matches!(
        Strkey::from_string(value.trim()),
        Ok(Strkey::PrivateKeyEd25519(_))
    )
}

fn is_stellar_address(value: &str) -> bool {
    matches!(
        Strkey::from_string(value.trim()),
        Ok(Strkey::PublicKeyEd25519(_)) | Ok(Strkey::Contract(_))
    )
}

fn is_bytes_literal(value: &str) -> bool {
    let trimmed = value.trim().trim_start_matches("0x").trim_start_matches("0X");
    trimmed.len() % 2 == 0
        && !trimmed.is_empty()
        && trimmed.chars().all(|char| char.is_ascii_hexdigit())
}

fn hex_to_bytes(value: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let trimmed = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    hex::decode(trimmed).with_context(|| format!("invalid hex argument `{value}`"))
}

fn normalize_rpc_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.ends_with("/soroban/rpc") {
        trimmed.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn network_id(passphrase: &str) -> [u8; 32] {
    Sha256::digest(passphrase.as_bytes()).into()
}

fn network_passphrase(network: &str) -> &str {
    match network.trim().to_ascii_lowercase().as_str() {
        "testnet" => "Test SDF Network ; September 2015",
        "mainnet" | "public" | "pubnet" => "Public Global Stellar Network ; September 2015",
        "futurenet" => "Test SDF Future Network ; October 2022",
        other if other.contains("network ;") => network.trim(),
        _ => network.trim(),
    }
}
