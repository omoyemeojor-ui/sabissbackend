INSERT INTO wallet_accounts (id, user_id, wallet_address, network)
VALUES ($1, $2, $3, $4)
RETURNING
    wallet_address,
    network,
    account_kind,
    wallet_status,
    wallet_standard,
    owner_provider,
    owner_ref,
    sponsor_address,
    relayer_kind,
    relayer_url,
    factory_contract_id,
    web_auth_contract_id,
    web_auth_domain,
    deployed_at,
    last_authenticated_at,
    created_at
