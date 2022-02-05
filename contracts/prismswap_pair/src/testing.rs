use crate::contract::{assert_max_spread, execute, instantiate, query, reply};
use crate::error::ContractError;
use crate::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Coin, ContractResult, CosmosMsg, Decimal, Reply,
    ReplyOn, Response, StdError, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use prismswap::asset::{Asset, AssetInfo, PairInfo};
use prismswap::pair::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolResponse, QueryMsg,
    ReverseSimulationResponse, SimulationResponse,
};
use prismswap::token::InstantiateMsg as TokenInstantiateMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &String::from("asset0000"),
        &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(123u128))],
    )]);

    // failure - invalid token addr
    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("te")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err(
            "Invalid input: human address too short"
        ))
    );

    // failure - doubling assets
    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Native("uusd".to_string()),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::DoublingAssets {});

    // success
    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    // we can just call .unwrap() to assert this was a success
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg {
            msg: WasmMsg::Instantiate {
                code_id: 10u64,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "UUSD-PRISM-LP".to_string(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                })
                .unwrap(),
                funds: vec![],
                label: "".to_string(),
                admin: None,
            }
            .into(),
            gas_limit: None,
            id: 1,
            reply_on: ReplyOn::Success,
        }]
    );

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // it worked, let's query the state
    let pair_info: PairInfo =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Pair {}).unwrap()).unwrap();

    assert_eq!("liquidity0000", pair_info.liquidity_token.as_str());
    assert_eq!(
        pair_info.asset_infos,
        [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000"))
        ]
    );

    let config_response: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        config_response,
        ConfigResponse {
            pair_info,
            factory: Addr::unchecked("factory0000"),
        }
    )
}

#[test]
fn provide_liquidity() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200u128),
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset0000".to_string(), &[]),
    ]);

    // failure - invalid token addr
    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("te")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err(
            "Invalid input: human address too short"
        ))
    );

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // provide more liquidity 1:2, which is not proportional to 1:1,
    // then it must accept 1:1 and treat left amount as donation
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                200u128 + 200u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(200u128))],
        ),
    ]);

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(200u128),
            },
        ],
        slippage_tolerance: None,
        receiver: Some("staking0000".to_string()), // try changing receiver
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );

    // only accept 100, then 50 share will be generated with 100 * (100 / 200)
    let res: Response = execute(deps.as_mut(), env, info, msg).unwrap();
    let transfer_from_msg = res.messages.get(0).expect("no message");
    let mint_msg = res.messages.get(1).expect("no message");
    assert_eq!(
        transfer_from_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        mint_msg,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "staking0000".to_string(), // LP tokens sent to specified receiver
                amount: Uint128::from(50u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    // check wrong argument
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(50u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Native token balance mismatch between the argument and the transferred".to_string()
        ),
        _ => panic!("Must return generic error"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100u128 + 100u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
    ]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(98u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::MaxSlippageAssertion {} => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128 + 98u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // failed because the price is under slippage_tolerance
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(98u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(98u128),
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::MaxSlippageAssertion {} => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(
                100u128 + 100u128, /* user deposit must be pre-applied */
            ),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(99u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // initialize token balance to 1:1
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128 + 99u128 /* user deposit must be pre-applied */),
        }],
    )]);

    // successfully provides
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: Uint128::from(99u128),
            },
        ],
        slippage_tolerance: Some(Decimal::percent(1)),
        receiver: None,
    };

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(99u128),
        }],
    );
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn withdraw_liquidity() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&"addr0000".to_string(), &Uint128::from(100u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // withdraw liquidity
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let log_withdrawn_share = res.attributes.get(2).expect("no log");
    let log_refund_assets = res.attributes.get(3).expect("no log");
    let msg_refund_0 = res.messages.get(0).expect("no message");
    let msg_refund_1 = res.messages.get(1).expect("no message");
    let msg_burn_liquidity = res.messages.get(2).expect("no message");
    assert_eq!(
        msg_refund_0,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100u128),
            }],
        }))
    );
    assert_eq!(
        msg_refund_1,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );
    assert_eq!(
        msg_burn_liquidity,
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "liquidity0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::from(100u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    assert_eq!(
        log_withdrawn_share,
        &attr("withdrawn_share", 100u128.to_string())
    );
    assert_eq!(
        log_refund_assets,
        &attr("refund_assets", "native:uusd:100, cw20:asset0000:100")
    );
}

#[test]
fn try_native_to_token() {
    let total_share = Uint128::from(30000000000u128);
    let asset_pool_amount = Uint128::from(20000000000u128);
    let collateral_pool_amount = Uint128::from(30000000000u128);
    let exchange_rate: Decimal = Decimal::from_ratio(asset_pool_amount, collateral_pool_amount);
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount + offer_amount, /* user deposit must be pre-applied */
    }]);

    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_pool_amount)],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // normal swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::Native("uusd".to_string()),
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: offer_amount,
        }],
    );
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    let msg_transfer = res.messages.get(0).expect("no message");
    let msg_collector = res.messages.get(1).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // 952.380952 = 20000 - 20000 * 30000 / (30000 + 1500)
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_commission_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.3%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_commission_amount)
        .unwrap();
    let expected_protocol_fee = expected_commission_amount.multiply_ratio(1u128, 2u128); // 50%

    // check simulation res
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: collateral_pool_amount, /* user deposit must be pre-applied */
        }],
    )]);

    let simulation_res: SimulationResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Simulation {
                offer_asset: Asset {
                    info: AssetInfo::Native("uusd".to_string()),
                    amount: offer_amount,
                },
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(expected_return_amount, simulation_res.return_amount);
    assert_eq!(expected_commission_amount, simulation_res.commission_amount);
    assert_eq!(expected_spread_amount, simulation_res.spread_amount);

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ReverseSimulation {
                ask_asset: Asset {
                    info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                    amount: expected_return_amount,
                },
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert!(
        (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs()
            < 3i128
    );
    assert!(
        (expected_commission_amount.u128() as i128
            - reverse_simulation_res.commission_amount.u128() as i128)
            .abs()
            < 3i128
    );
    assert!(
        (expected_spread_amount.u128() as i128
            - reverse_simulation_res.spread_amount.u128() as i128)
            .abs()
            < 3i128
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("sender", "addr0000"),
            attr("receiver", "addr0000"),
            attr("offer_asset", "native:uusd"),
            attr("ask_asset", "cw20:asset0000"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", expected_return_amount.to_string()),
            attr("spread_amount", expected_spread_amount.to_string()),
            attr("commission_amount", expected_commission_amount.to_string()),
            attr("protocol_fee_amount", expected_protocol_fee.to_string()),
        ]
    );

    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: expected_return_amount,
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_transfer,
    );
    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "collector".to_string(),
                amount: expected_protocol_fee,
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_collector,
    );
}

#[test]
fn try_token_to_native() {
    let total_share = Uint128::from(20000000000u128);
    let asset_pool_amount = Uint128::from(30000000000u128);
    let collateral_pool_amount = Uint128::from(20000000000u128);
    let exchange_rate = Decimal::from_ratio(collateral_pool_amount, asset_pool_amount);
    let offer_amount = Uint128::from(1500000000u128);

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: collateral_pool_amount,
    }]);
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(
                &MOCK_CONTRACT_ADDR.to_string(),
                &(asset_pool_amount + offer_amount),
            )],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // unauthorized access; can not execute swap directly for token swap
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // failure - invalid token addr
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::Cw20(Addr::unchecked("te")),
            amount: offer_amount,
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err(
            "Invalid input: human address too short"
        ))
    );

    // normal sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env();
    let info = mock_info("asset0000", &[]);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    let msg_transfer = res.messages.get(0).expect("no message");
    let msg_collector = res.messages.get(1).expect("no message");

    // current price is 1.5, so expected return without spread is 1000
    // 952.380952 = 20000 - 20000 * 30000 / (30000 + 1500)
    let expected_ret_amount = Uint128::from(952_380_952u128);
    let expected_spread_amount = (offer_amount * exchange_rate)
        .checked_sub(expected_ret_amount)
        .unwrap();
    let expected_commission_amount = expected_ret_amount.multiply_ratio(3u128, 1000u128); // 0.3%
    let expected_return_amount = expected_ret_amount
        .checked_sub(expected_commission_amount)
        .unwrap();
    let expected_protocol_fee_amount = expected_commission_amount.multiply_ratio(1u128, 2u128); // 50%

    // check simulation res
    // return asset token balance as normal
    deps.querier.with_token_balances(&[
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share)],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &(asset_pool_amount))],
        ),
    ]);

    let simulation_res: SimulationResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Simulation {
                offer_asset: Asset {
                    amount: offer_amount,
                    info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(expected_return_amount, simulation_res.return_amount);
    assert_eq!(expected_commission_amount, simulation_res.commission_amount);
    assert_eq!(expected_spread_amount, simulation_res.spread_amount);

    // check reverse simulation res
    let reverse_simulation_res: ReverseSimulationResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ReverseSimulation {
                ask_asset: Asset {
                    amount: expected_return_amount,
                    info: AssetInfo::Native("uusd".to_string()),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        (offer_amount.u128() as i128 - reverse_simulation_res.offer_amount.u128() as i128).abs()
            < 3i128
    );
    assert!(
        (expected_commission_amount.u128() as i128
            - reverse_simulation_res.commission_amount.u128() as i128)
            .abs()
            < 3i128
    );
    assert!(
        (expected_spread_amount.u128() as i128
            - reverse_simulation_res.spread_amount.u128() as i128)
            .abs()
            < 3i128
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "swap"),
            attr("sender", "addr0000"),
            attr("receiver", "addr0000"),
            attr("offer_asset", "cw20:asset0000"),
            attr("ask_asset", "native:uusd"),
            attr("offer_amount", offer_amount.to_string()),
            attr("return_amount", expected_return_amount.to_string()),
            attr("spread_amount", expected_spread_amount.to_string()),
            attr("commission_amount", expected_commission_amount.to_string()),
            attr(
                "protocol_fee_amount",
                expected_protocol_fee_amount.to_string()
            ),
        ]
    );

    assert_eq!(
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: expected_return_amount,
            }],
        })),
        msg_transfer,
    );
    assert_eq!(
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "collector".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: expected_protocol_fee_amount,
            }],
        })),
        msg_collector,
    );

    // failed due to non asset token contract try to execute sell
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
        })
        .unwrap(),
    });
    let env = mock_env();
    let info = mock_info("liquidity0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_max_spread() {
    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1200000000u128),
        Uint128::from(989999u128),
        Uint128::zero(),
    )
    .unwrap_err();

    assert_max_spread(
        Some(Decimal::from_ratio(1200u128, 1u128)),
        Some(Decimal::percent(1)),
        Uint128::from(1200000000u128),
        Uint128::from(990000u128),
        Uint128::zero(),
    )
    .unwrap();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(989999u128),
        Uint128::from(10001u128),
    )
    .unwrap_err();

    assert_max_spread(
        None,
        Some(Decimal::percent(1)),
        Uint128::zero(),
        Uint128::from(990000u128),
        Uint128::from(10000u128),
    )
    .unwrap();
}

#[test]
fn test_query_pool() {
    let total_share_amount = Uint128::from(111u128);
    let asset_0_amount = Uint128::from(222u128);
    let asset_1_amount = Uint128::from(333u128);
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: asset_0_amount,
    }]);

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &asset_1_amount)],
        ),
        (
            &"liquidity0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &total_share_amount)],
        ),
    ]);

    let msg = InstantiateMsg {
        asset_infos: [
            AssetInfo::Native("uusd".to_string()),
            AssetInfo::Cw20(Addr::unchecked("asset0000")),
        ],
        token_code_id: 10u64,
        factory: Addr::unchecked("factory0000"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // store liquidity token
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(
                vec![
                    10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                ]
                .into(),
            ),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    let res: PoolResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Pool {}).unwrap()).unwrap();
    assert_eq!(
        res.assets,
        [
            Asset {
                info: AssetInfo::Native("uusd".to_string()),
                amount: asset_0_amount
            },
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                amount: asset_1_amount
            }
        ]
    );
    assert_eq!(res.total_share, total_share_amount);
}
