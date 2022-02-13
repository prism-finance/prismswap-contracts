use std::str::FromStr;

use crate::contract::{execute, instantiate, query, reply};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};

use crate::state::{pair_key, TmpPairInfo, TMP_PAIR_INFO};

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, ContractResult, Decimal, MemoryStorage, OwnedDeps, Reply,
    ReplyOn, StdError, SubMsg, SubMsgExecutionResponse, WasmMsg,
};
use prismswap::asset::{AssetInfo, PairInfo};
use prismswap::factory::{
    ConfigResponse, ExecuteMsg, FeeConfig, FeeInfoResponse, InstantiateMsg, PairConfigResponse,
    PairsConfigResponse, PairsResponse, QueryMsg, DEFAULT_PROTOCOL_FEE, DEFAULT_TOTAL_FEE,
    MAX_PROTOCOL_FEE, MAX_TOTAL_FEE,
};
use prismswap::pair::InstantiateMsg as PairInstantiateMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("owner0000".to_string(), config_res.owner);
    assert_eq!("collector0000".to_string(), config_res.collector);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let info = mock_info("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(Addr::unchecked("addr0001")),
        pair_code_id: None,
        token_code_id: None,
        collector: None,
        pairs_admin: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("collector0000".to_string(), config_res.collector);
    assert_eq!("addr0001".to_string(), config_res.owner);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
        collector: Some(Addr::unchecked("collector0001")),
        pairs_admin: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);
    assert_eq!("collector0001".to_string(), config_res.collector);

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: None,
        token_code_id: None,
        collector: None,
        pairs_admin: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn create_pair() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: None,
    };

    // unauthorized attempt
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "cw20:asset0000-cw20:asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    factory: Addr::unchecked(MOCK_CONTRACT_ADDR),
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "".to_string(),
                admin: Some("admin0000".to_string())
            }
            .into()
        },]
    );

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            fee_config: FeeConfig::default(),
            pair_key: pair_key(&asset_infos),
        }
    );
}

#[test]
fn reply_test() {
    let mut deps = mock_dependencies(&[]);

    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let pair_key = pair_key(&asset_infos);
    TMP_PAIR_INFO
        .save(
            &mut deps.storage,
            &TmpPairInfo {
                fee_config: FeeConfig::default(),
                pair_key,
            },
        )
        .unwrap();

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(vec![10, 8, 112, 97, 105, 114, 48, 48, 48, 48].into()),
        }),
    };

    deps.querier.with_pairs(&[(
        &"pair0000".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: Addr::unchecked("pair0000"),
            liquidity_token: Addr::unchecked("liquidity0000"),
        },
    )]);

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();

    let pair_res: PairInfo = from_binary(&query_res).unwrap();
    assert_eq!(
        pair_res,
        PairInfo {
            liquidity_token: Addr::unchecked("liquidity0000"),
            contract_addr: Addr::unchecked("pair0000".to_string()),
            asset_infos,
        }
    );
}

// helper to simulate a pair creation by constructing a reply message
// for the contract  with the associated contract address, pair key, and fee config
fn simulate_pair_creation(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>,
    contract_addr: &str,
    asset_infos: &[AssetInfo; 2],
    fee_config: Option<FeeConfig>,
) {
    let pair_key = pair_key(asset_infos);
    TMP_PAIR_INFO
        .save(
            deps.as_mut().storage,
            &TmpPairInfo {
                pair_key,
                fee_config: fee_config.unwrap_or_default(),
            },
        )
        .unwrap();

    let mut bytes: Vec<u8> = vec![10];
    bytes.push(contract_addr.len().to_le_bytes()[0]);
    bytes.extend_from_slice(contract_addr.as_bytes());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(bytes.into()),
        }),
    };

    reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
}

#[test]
fn create_pair_2() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: None,
    };

    // unauthorized attempt
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    // set correct owner
    let info = mock_info("owner0000", &[]);

    // failure - invalid fee config
    let invalid_fee_msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: Some(FeeConfig {
            total_fee: Decimal::from_str(MAX_TOTAL_FEE).unwrap() + Decimal::one(),
            protocol_fee: Decimal::from_str(DEFAULT_PROTOCOL_FEE).unwrap(),
        }),
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), invalid_fee_msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("The given fee configuration is not valid")
    );

    // failure - invalid fee config
    let invalid_msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: Some(FeeConfig {
            total_fee: Decimal::from_str(DEFAULT_TOTAL_FEE).unwrap(),
            protocol_fee: Decimal::from_str(MAX_PROTOCOL_FEE).unwrap() + Decimal::one(),
        }),
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), invalid_msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("The given fee configuration is not valid")
    );

    // failure - invalid token
    let asset_infos_bad = [
        AssetInfo::Cw20(Addr::unchecked("te")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];
    let invalid_msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos_bad,
        fee_config: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), invalid_msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid input: human address too short")
    );

    // success
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "cw20:asset0000-cw20:asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    factory: Addr::unchecked(MOCK_CONTRACT_ADDR),
                    asset_infos: asset_infos.clone(),
                    token_code_id: 123u64,
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "".to_string(),
                admin: Some("admin0000".to_string()),
            }
            .into()
        },]
    );

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            fee_config: FeeConfig::default(),
            pair_key: pair_key(&asset_infos),
        }
    );

    simulate_pair_creation(&mut deps, "pairaddr0001", &asset_infos, None);
    deps.querier.with_pairs(&[(
        &"pairaddr0001".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: Addr::unchecked("pairaddr0001"),
            liquidity_token: Addr::unchecked("liquidity0001"),
        },
    )]);

    let fee_config_default = FeeConfig {
        total_fee: Decimal::from_str(DEFAULT_TOTAL_FEE).unwrap(),
        protocol_fee: Decimal::from_str(DEFAULT_PROTOCOL_FEE).unwrap(),
    };

    // query pair config, we should get default values
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PairConfig {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();
    let pair_config: PairConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(pair_config.fee_config, fee_config_default);

    // failure - pair already exists
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(err, StdError::generic_err("Pair already exists"));

    // create new pair, this time with a valid FeeConfig
    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0002")),
        AssetInfo::Cw20(Addr::unchecked("asset0003")),
    ];
    let custom_fee_config = FeeConfig {
        total_fee: Decimal::from_str("0.004").unwrap(),
        protocol_fee: Decimal::from_str("0.075").unwrap(),
    };

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: Some(custom_fee_config.clone()),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    simulate_pair_creation(
        &mut deps,
        "pairaddr0001",
        &asset_infos,
        Some(custom_fee_config.clone()),
    );
    deps.querier.with_pairs(&[(
        &"pairaddr0001".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: Addr::unchecked("pairaddr0001"),
            liquidity_token: Addr::unchecked("liquidity0001"),
        },
    )]);

    // query pair config, we should get cuustom values
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PairConfig {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();
    let pair_config: PairConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(pair_config.fee_config, custom_fee_config);
}

#[test]
fn test_update_pair_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let fee_config_default = FeeConfig {
        total_fee: Decimal::from_str(DEFAULT_TOTAL_FEE).unwrap(),
        protocol_fee: Decimal::from_str(DEFAULT_PROTOCOL_FEE).unwrap(),
    };

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: None,
    };

    // successful create pair
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // simulate created pair
    simulate_pair_creation(&mut deps, "pairaddr0001", &asset_infos, None);
    deps.querier.with_pairs(&[(
        &"pairaddr0001".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: Addr::unchecked("pairaddr0001"),
            liquidity_token: Addr::unchecked("liquidity0001"),
        },
    )]);

    // query pair config
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PairConfig {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();
    let pair_config: PairConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(pair_config.fee_config, fee_config_default);

    // update fee config to new values
    let fee_config_updated = FeeConfig {
        total_fee: Decimal::from_str("0.04").unwrap(),
        protocol_fee: Decimal::from_str("0.75").unwrap(),
    };
    let msg = ExecuteMsg::UpdatePairConfig {
        asset_infos: asset_infos.clone(),
        fee_config: fee_config_updated.clone(),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_pair_config"),]);

    // query new pair config, verify updated correctly
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PairConfig {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();
    let pair_config: PairConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(pair_config.fee_config, fee_config_updated);

    // failure - unauthorized
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    // failure - invalid fee config
    let info = mock_info("owner0000", &[]);
    let invalid_msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: Some(FeeConfig {
            total_fee: Decimal::from_str(DEFAULT_TOTAL_FEE).unwrap(),
            protocol_fee: Decimal::from_str(MAX_PROTOCOL_FEE).unwrap() + Decimal::one(),
        }),
    };
    let err = execute(deps.as_mut(), mock_env(), info, invalid_msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("The given fee configuration is not valid")
    );

    // failure - no pair exists
    let info = mock_info("owner0000", &[]);
    let asset_infos_bad = [
        AssetInfo::Cw20(Addr::unchecked("asset0002")),
        AssetInfo::Cw20(Addr::unchecked("asset0003")),
    ];
    let msg_bad = ExecuteMsg::UpdatePairConfig {
        asset_infos: asset_infos_bad,
        fee_config: fee_config_updated.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg_bad).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("There is no pair registered with the provided info")
    );

    // failure - invalid token
    let asset_infos_bad = [
        AssetInfo::Cw20(Addr::unchecked("te")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];
    let msg_bad = ExecuteMsg::UpdatePairConfig {
        asset_infos: asset_infos_bad,
        fee_config: fee_config_updated,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg_bad).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid input: human address too short")
    );
}

#[test]
fn test_deregister() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        fee_config: None,
    };

    // successful create pair
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // simulate created pair
    simulate_pair_creation(&mut deps, "pairaddr0001", &asset_infos, None);
    deps.querier.with_pairs(&[(
        &"pairaddr0001".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: Addr::unchecked("pairaddr0001"),
            liquidity_token: Addr::unchecked("liquidity0001"),
        },
    )]);

    // query pair config, verify successful creation
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PairConfig {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();
    let pair_config: PairConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(pair_config.fee_config, FeeConfig::default());

    // failure - unauthorized
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::Deregister {
        asset_infos: asset_infos.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    // failure - no pair exists
    let info = mock_info("owner0000", &[]);
    let asset_infos_bad = [
        AssetInfo::Cw20(Addr::unchecked("asset0002")),
        AssetInfo::Cw20(Addr::unchecked("asset0003")),
    ];
    let msg_bad = ExecuteMsg::Deregister {
        asset_infos: asset_infos_bad,
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg_bad).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("There is no pair registered with the provided info")
    );

    // failure - invalid token
    let asset_infos_bad = [
        AssetInfo::Cw20(Addr::unchecked("te")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];
    let msg_bad = ExecuteMsg::Deregister {
        asset_infos: asset_infos_bad,
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg_bad).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid input: human address too short")
    );

    // success
    let msg = ExecuteMsg::Deregister {
        asset_infos: asset_infos.clone(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages, vec![]);
    assert_eq!(res.attributes, vec![attr("action", "deregister")]);
}

#[test]
fn test_queries() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
        owner: Addr::unchecked("owner0000"),
        collector: Addr::unchecked("collector0000"),
        pairs_admin: Addr::unchecked("admin0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos1 = [
        AssetInfo::Cw20(Addr::unchecked("asset0000")),
        AssetInfo::Cw20(Addr::unchecked("asset0001")),
    ];

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos1.clone(),
        fee_config: None,
    };

    // successful create pair
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // simulate created pair
    simulate_pair_creation(&mut deps, "pairaddr0001", &asset_infos1, None);

    // create new pair, this time with a valid FeeConfig
    let asset_infos2 = [
        AssetInfo::Cw20(Addr::unchecked("asset0002")),
        AssetInfo::Cw20(Addr::unchecked("asset0003")),
    ];
    let fee_config2 = FeeConfig {
        total_fee: Decimal::from_str("0.004").unwrap(),
        protocol_fee: Decimal::from_str("0.075").unwrap(),
    };

    let msg = ExecuteMsg::CreatePair {
        asset_infos: asset_infos2.clone(),
        fee_config: Some(fee_config2.clone()),
    };

    // successful create pair
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // simulate created pair
    simulate_pair_creation(
        &mut deps,
        "pairaddr0002",
        &asset_infos2,
        Some(fee_config2.clone()),
    );

    deps.querier.with_pairs(&[
        (
            &"pairaddr0001".to_string(),
            &PairInfo {
                asset_infos: asset_infos1.clone(),
                contract_addr: Addr::unchecked("pairaddr0001"),
                liquidity_token: Addr::unchecked("liquidity0001"),
            },
        ),
        (
            &"pairaddr0002".to_string(),
            &PairInfo {
                asset_infos: asset_infos2.clone(),
                contract_addr: Addr::unchecked("pairaddr0002"),
                liquidity_token: Addr::unchecked("liquidity0002"),
            },
        ),
    ]);

    // pairs query
    let pairs_response: PairsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Pairs {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_response,
        PairsResponse {
            pairs: vec![
                PairInfo {
                    asset_infos: asset_infos1.clone(),
                    contract_addr: Addr::unchecked("pairaddr0001"),
                    liquidity_token: Addr::unchecked("liquidity0001")
                },
                PairInfo {
                    asset_infos: asset_infos2.clone(),
                    contract_addr: Addr::unchecked("pairaddr0002"),
                    liquidity_token: Addr::unchecked("liquidity0002")
                },
            ]
        }
    );

    // pairs query limit 1
    let pairs_response: PairsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Pairs {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_response,
        PairsResponse {
            pairs: vec![PairInfo {
                asset_infos: asset_infos1.clone(),
                contract_addr: Addr::unchecked("pairaddr0001"),
                liquidity_token: Addr::unchecked("liquidity0001")
            },]
        }
    );

    // pairs query start after first
    let pairs_response: PairsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Pairs {
                start_after: Some(asset_infos1.clone()),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_response,
        PairsResponse {
            pairs: vec![PairInfo {
                asset_infos: asset_infos2.clone(),
                contract_addr: Addr::unchecked("pairaddr0002"),
                liquidity_token: Addr::unchecked("liquidity0002")
            },]
        }
    );

    // query fee info 1
    let fee_info_response: FeeInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::FeeInfo {
                asset_infos: asset_infos1.clone(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fee_info_response,
        FeeInfoResponse {
            fee_config: FeeConfig::default(),
            collector: Addr::unchecked("collector0000")
        }
    );

    // query fee info 2
    let fee_info_response: FeeInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::FeeInfo {
                asset_infos: asset_infos2.clone(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fee_info_response,
        FeeInfoResponse {
            fee_config: fee_config2.clone(),
            collector: Addr::unchecked("collector0000")
        }
    );

    // pairs config query
    let pairs_config_response: PairsConfigResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PairsConfig {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_config_response,
        PairsConfigResponse {
            pairs: vec![
                PairConfigResponse {
                    pair_info: PairInfo {
                        asset_infos: asset_infos1.clone(),
                        contract_addr: Addr::unchecked("pairaddr0001"),
                        liquidity_token: Addr::unchecked("liquidity0001")
                    },
                    fee_config: FeeConfig::default()
                },
                PairConfigResponse {
                    pair_info: PairInfo {
                        asset_infos: asset_infos2.clone(),
                        contract_addr: Addr::unchecked("pairaddr0002"),
                        liquidity_token: Addr::unchecked("liquidity0002")
                    },
                    fee_config: fee_config2.clone()
                },
            ]
        }
    );

    // pairs config query limit 1
    let pairs_config_response: PairsConfigResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PairsConfig {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_config_response,
        PairsConfigResponse {
            pairs: vec![PairConfigResponse {
                pair_info: PairInfo {
                    asset_infos: asset_infos1.clone(),
                    contract_addr: Addr::unchecked("pairaddr0001"),
                    liquidity_token: Addr::unchecked("liquidity0001")
                },
                fee_config: FeeConfig::default()
            },]
        }
    );

    // pairs config query start after first
    let pairs_config_response: PairsConfigResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PairsConfig {
                start_after: Some(asset_infos1.clone()),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        pairs_config_response,
        PairsConfigResponse {
            pairs: vec![PairConfigResponse {
                pair_info: PairInfo {
                    asset_infos: asset_infos2.clone(),
                    contract_addr: Addr::unchecked("pairaddr0002"),
                    liquidity_token: Addr::unchecked("liquidity0002")
                },
                fee_config: fee_config2
            },]
        }
    );
}
