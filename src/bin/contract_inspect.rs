use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client;
use sabissbackend::config::environment::Environment;
use serde::{Deserialize, Serialize};
use stellar_strkey::Strkey;
use stellar_xdr::curr::{
    ContractDataDurability, ContractId, Hash, LedgerKey, LedgerKeyContractCode,
    LedgerEntryData, LedgerKeyContractData, Limits, PublicKey, ReadXdr, ScAddress, ScVal,
    Uint256, WriteXdr,
};

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
}

#[derive(Serialize)]
struct GetLedgerEntriesParams {
    keys: Vec<String>,
    #[serde(rename = "xdrFormat")]
    xdr_format: &'static str,
}

#[derive(Deserialize)]
struct GetLedgerEntriesResult {
    entries: Vec<LedgerEntryEnvelope>,
}

#[derive(Deserialize)]
struct LedgerEntryEnvelope {
    xdr: String,
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let env = Environment::load().context("failed to load environment")?;
    let contract_id = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("usage: cargo run --bin contract_inspect -- <contract-id>"))?;

    let runtime = tokio::runtime::Runtime::new().context("failed to build Tokio runtime")?;
    runtime.block_on(run(&env, &contract_id))
}

async fn run(env: &Environment, contract_id: &str) -> Result<()> {
    let client = Client::builder()
        .build()
        .context("failed to build HTTP client")?;

    let instance_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: parse_contract_address(contract_id)?,
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });
    let instance_key = BASE64.encode(instance_key.to_xdr(Limits::none())?);
    println!("instance_key={instance_key}");

    let result: GetLedgerEntriesResult = send_request(
        &client,
        &env.rpc_url,
        "getLedgerEntries",
        GetLedgerEntriesParams {
            keys: vec![instance_key],
            xdr_format: "base64",
        },
    )
    .await?;
    let instance_entry = result
        .entries
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("contract instance entry was not found"))?;
    let instance_bytes = BASE64
        .decode(instance_entry.xdr)
        .context("failed to decode contract instance entry")?;
    let instance_data =
        LedgerEntryData::from_xdr(instance_bytes, Limits::none()).context("failed to decode contract instance XDR")?;
    println!("contract_instance={instance_data:#?}");

    let wasm_hash = extract_wasm_hash(&instance_data)?;
    let code_key = LedgerKey::ContractCode(LedgerKeyContractCode {
        hash: Hash(
            hex::decode(wasm_hash)?
                .try_into()
                .map_err(|_| anyhow!("invalid wasm hash length"))?,
        ),
    });
    let code_key = BASE64.encode(code_key.to_xdr(Limits::none())?);
    println!("code_key={code_key}");
    let result: GetLedgerEntriesResult = send_request(
        &client,
        &env.rpc_url,
        "getLedgerEntries",
        GetLedgerEntriesParams {
            keys: vec![code_key],
            xdr_format: "base64",
        },
    )
    .await?;
    let code_entry = result
        .entries
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("contract code entry was not found"))?;
    let code_bytes = BASE64
        .decode(code_entry.xdr)
        .context("failed to decode contract code entry")?;
    let code_data =
        LedgerEntryData::from_xdr(code_bytes, Limits::none()).context("failed to decode contract code XDR")?;
    println!("contract_code={code_data:#?}");
    if let LedgerEntryData::ContractCode(entry) = &code_data {
        let out_path = format!("/tmp/{contract_id}.wasm");
        std::fs::write(&out_path, entry.code.as_slice())
            .with_context(|| format!("failed to write `{out_path}`"))?;
        println!("wasm_path={out_path}");
    }

    Ok(())
}

async fn send_request<TReq, TRes>(
    client: &Client,
    rpc_url: &str,
    method: &'static str,
    params: TReq,
) -> Result<TRes>
where
    TReq: Serialize,
    TRes: for<'de> Deserialize<'de>,
{
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
    };
    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .with_context(|| format!("failed to call `{method}`"))?
        .error_for_status()
        .with_context(|| format!("`{method}` returned an error status"))?;
    let payload: JsonRpcResponse<TRes> = response
        .json()
        .await
        .with_context(|| format!("failed to decode `{method}` response"))?;
    if let Some(error) = payload.error {
        return Err(anyhow!("RPC error {}: {}", error.code, error.message));
    }
    payload
        .result
        .ok_or_else(|| anyhow!("`{method}` response did not include a result"))
}

fn parse_contract_address(contract_id: &str) -> Result<ScAddress> {
    match Strkey::from_string(contract_id).context("invalid contract id")? {
        Strkey::Contract(contract) => Ok(ScAddress::Contract(ContractId(Hash(contract.0)))),
        Strkey::PublicKeyEd25519(public_key) => Ok(ScAddress::Account(stellar_xdr::curr::AccountId(
            PublicKey::PublicKeyTypeEd25519(Uint256(public_key.0)),
        ))),
        _ => Err(anyhow!("unsupported address `{contract_id}`")),
    }
}

fn extract_wasm_hash(instance_data: &LedgerEntryData) -> Result<String> {
    let LedgerEntryData::ContractData(entry) = instance_data else {
        return Err(anyhow!("ledger entry data was not contract data"));
    };
    let ScVal::ContractInstance(instance) = &entry.val else {
        return Err(anyhow!("contract data entry did not contain a contract instance"));
    };
    let stellar_xdr::curr::ContractExecutable::Wasm(hash) = &instance.executable else {
        return Err(anyhow!("contract instance is not backed by Wasm"));
    };

    Ok(hex::encode(hash.0))
}
