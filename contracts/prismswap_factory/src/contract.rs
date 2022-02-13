#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};

use crate::migration::migrate_config;
use crate::parse_reply::parse_reply_instantiate_data;
use crate::querier::query_pair_info;
use crate::state::{
    pair_key, read_pairs, Config, PairConfig, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO,
};

use prismswap::asset::{AssetInfo, PairInfo, PrismSwapAssetInfo};
use prismswap::factory::{
    ConfigResponse, ExecuteMsg, FeeConfig, FeeInfoResponse, InstantiateMsg, MigrateMsg,
    PairConfigResponse, PairsConfigResponse, PairsResponse, QueryMsg,
};
use prismswap::pair::InstantiateMsg as PairInstantiateMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
        collector: deps.api.addr_validate(msg.collector.as_str())?,
        pairs_admin: deps.api.addr_validate(msg.pairs_admin.as_str())?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
            collector,
            pairs_admin,
        } => execute_update_config(
            deps,
            info,
            owner,
            token_code_id,
            pair_code_id,
            collector,
            pairs_admin,
        ),
        ExecuteMsg::CreatePair {
            asset_infos,
            fee_config,
        } => {
            asset_infos[0].check(deps.api)?;
            asset_infos[1].check(deps.api)?;
            execute_create_pair(deps, info, env, asset_infos, fee_config)
        }
        ExecuteMsg::UpdatePairConfig {
            asset_infos,
            fee_config,
        } => {
            asset_infos[0].check(deps.api)?;
            asset_infos[1].check(deps.api)?;
            execute_update_pair_config(deps, info, asset_infos, fee_config)
        }
        ExecuteMsg::Deregister { asset_infos } => {
            asset_infos[0].check(deps.api)?;
            asset_infos[1].check(deps.api)?;
            execute_deregister(deps, info, asset_infos)
        }
    }
}

// Only owner can execute it
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
    collector: Option<Addr>,
    pairs_admin: Option<Addr>,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        deps.api.addr_validate(owner.as_str())?;
        config.owner = owner;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }

    if let Some(collector) = collector {
        deps.api.addr_validate(collector.as_str())?;
        config.collector = collector;
    }

    if let Some(pairs_admin) = pairs_admin {
        deps.api.addr_validate(pairs_admin.as_str())?;
        config.pairs_admin = pairs_admin;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Only owner can create pairs
pub fn execute_create_pair(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    asset_infos: [AssetInfo; 2],
    fee_config: Option<FeeConfig>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    // validate the given fee configuration
    let fee_config: FeeConfig = fee_config.unwrap_or_default();
    if !fee_config.is_valid() {
        return Err(StdError::generic_err(
            "The given fee configuration is not valid",
        ));
    }

    let pair_key = pair_key(&asset_infos);
    if PAIRS.may_load(deps.storage, &pair_key)?.is_some() {
        return Err(StdError::generic_err("Pair already exists"));
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            fee_config,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{}-{}", asset_infos[0], asset_infos[1])),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: vec![],
                admin: Some(config.pairs_admin.to_string()),
                label: "".to_string(),
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    factory: env.contract.address,
                })?,
            }
            .into(),
            reply_on: ReplyOn::Success,
        }))
}

// Only owner can execute it
pub fn execute_update_pair_config(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    fee_config: FeeConfig,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    // validate the given fee configuration
    if !fee_config.is_valid() {
        return Err(StdError::generic_err(
            "The given fee configuration is not valid",
        ));
    }

    let pair_key = pair_key(&asset_infos);
    let mut pair_config: PairConfig = PAIRS
        .load(deps.storage, &pair_key)
        .map_err(|_| StdError::generic_err("There is no pair registered with the provided info"))?;

    pair_config.fee_config = fee_config;

    PAIRS.save(deps.storage, &pair_key, &pair_config)?;

    Ok(Response::new().add_attribute("action", "update_pair_config"))
}

// Only owner can execute it
pub fn execute_deregister(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let pair_key = pair_key(&asset_infos);

    // check if pair exists
    PAIRS
        .load(deps.storage, &pair_key)
        .map_err(|_| StdError::generic_err("There is no pair registered with the provided info"))?;

    // delete the pair from storage
    PAIRS.remove(deps.storage, &pair_key);

    Ok(Response::new().add_attribute("action", "deregister"))
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res = parse_reply_instantiate_data(msg)
        .map_err(|err| StdError::generic_err(format!("{}", err)))?;
    let pair_contract = res.contract_address;

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairConfig {
            pair_address: deps.api.addr_validate(&pair_contract)?,
            fee_config: tmp_pair_info.fee_config,
        },
    )?;

    Ok(Response::new().add_attributes(vec![("pair_contract_addr", pair_contract)]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&query_pairs(deps, start_after, limit)?)
        }
        QueryMsg::FeeInfo { asset_infos } => to_binary(&query_fee_config(deps, asset_infos)?),
        QueryMsg::PairConfig { asset_infos } => to_binary(&query_pair_config(deps, asset_infos)?),
        QueryMsg::PairsConfig { start_after, limit } => {
            to_binary(&query_pairs_config(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: config.owner,
        token_code_id: config.token_code_id,
        pair_code_id: config.pair_code_id,
        collector: config.collector,
        pairs_admin: config.pairs_admin,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&asset_infos);
    let pair_config: PairConfig = PAIRS.load(deps.storage, &pair_key)?;

    query_pair_info(&deps.querier, &pair_config.pair_address)
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let pair_configs: Vec<PairConfig> = read_pairs(deps.storage, start_after, limit)?;

    let pair_infos: Vec<PairInfo> = pair_configs
        .iter()
        .map(|pair| query_pair_info(&deps.querier, &pair.pair_address))
        .collect::<StdResult<Vec<PairInfo>>>()?;

    Ok(PairsResponse { pairs: pair_infos })
}

pub fn query_fee_config(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<FeeInfoResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let pair_key = pair_key(&asset_infos);
    let fee_config: FeeConfig = match PAIRS.load(deps.storage, &pair_key) {
        Ok(config) => config.fee_config,
        _ => FeeConfig::default(),
    };

    Ok(FeeInfoResponse {
        collector: config.collector,
        fee_config,
    })
}

pub fn query_pair_config(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairConfigResponse> {
    let pair_key = pair_key(&asset_infos);
    let pair_config: PairConfig = PAIRS.load(deps.storage, &pair_key)?;

    let pair_info: PairInfo = query_pair_info(&deps.querier, &pair_config.pair_address)?;

    Ok(PairConfigResponse {
        pair_info,
        fee_config: pair_config.fee_config,
    })
}

pub fn query_pairs_config(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsConfigResponse> {
    let pair_configs: Vec<PairConfig> = read_pairs(deps.storage, start_after, limit)?;

    let res_items: Vec<PairConfigResponse> = pair_configs
        .iter()
        .map(|pair| {
            let pair_info: PairInfo = query_pair_info(&deps.querier, &pair.pair_address)?;

            Ok(PairConfigResponse {
                pair_info,
                fee_config: pair.fee_config.clone(),
            })
        })
        .collect::<StdResult<Vec<PairConfigResponse>>>()?;

    Ok(PairsConfigResponse { pairs: res_items })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let pairs_admin: Addr = deps.api.addr_validate(msg.pairs_admin.as_str())?;
    migrate_config(deps.storage, pairs_admin)?;

    Ok(Response::default())
}
