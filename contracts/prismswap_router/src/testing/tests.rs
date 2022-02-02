use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Coin, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg,
};

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use prismswap::asset::{Asset, AssetInfo};
use prismswap::pair::ExecuteMsg as PairExecuteMsg;
use prismswap::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use terra_cosmwasm::{create_swap_msg, create_swap_send_msg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        factory: Addr::unchecked("factory"),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("factory", config.factory.as_str());
}

#[test]
fn execute_swap_operations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"asset0002".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = InstantiateMsg {
        factory: Addr::unchecked("factory"),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "must provide operations"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "ukrw".to_string(),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                ask_asset_info: AssetInfo::Native("uluna".to_string()),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Native("uluna".to_string()),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0002")),
            },
        ],
        minimum_receive: Some(Uint128::from(1000000u128)),
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::NativeSwap {
                        offer_denom: "uusd".to_string(),
                        ask_denom: "ukrw".to_string(),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                        ask_asset_info: AssetInfo::Native("uluna".to_string()),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Native("uluna".to_string()),
                        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0002")),
                    },
                    to: Some(Addr::unchecked("addr0000")),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
                    asset_info: AssetInfo::Cw20(Addr::unchecked("asset0002")),
                    prev_balance: Uint128::zero(),
                    minimum_receive: Uint128::from(1000000u128),
                    receiver: Addr::unchecked("addr0000"),
                })
                .unwrap(),
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![
                SwapOperation::NativeSwap {
                    offer_denom: "uusd".to_string(),
                    ask_denom: "ukrw".to_string(),
                },
                SwapOperation::PrismSwap {
                    offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                    ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                },
                SwapOperation::PrismSwap {
                    offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                    ask_asset_info: AssetInfo::Native("uluna".to_string()),
                },
                SwapOperation::PrismSwap {
                    offer_asset_info: AssetInfo::Native("uluna".to_string()),
                    ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0002")),
                },
            ],
            minimum_receive: None,
            to: Some(Addr::unchecked("addr0002")),
        })
        .unwrap(),
    });

    let info = mock_info("asset0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::NativeSwap {
                        offer_denom: "uusd".to_string(),
                        ask_denom: "ukrw".to_string(),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                        ask_asset_info: AssetInfo::Native("uluna".to_string()),
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::PrismSwap {
                        offer_asset_info: AssetInfo::Native("uluna".to_string()),
                        ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0002")),
                    },
                    to: Some(Addr::unchecked("addr0002"),),
                })
                .unwrap(),
            }))
        ]
    );

    // failure - invalid token addr
    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "ukrw".to_string(),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0001")),
                ask_asset_info: AssetInfo::Native("uluna".to_string()),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Native("uluna".to_string()),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("te")),
            },
        ],
        minimum_receive: Some(Uint128::from(1000000u128)),
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid input: human address too short")
    );
}

#[test]
fn execute_swap_operation() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        factory: Addr::unchecked("factory"),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier
        .with_pairs(&[(&"uusdasset".to_string(), &"pair".to_string())]);
    deps.querier.with_balance(&[(
        MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            amount: Uint128::from(1000000u128),
            denom: "uusd".to_string(),
        }],
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::NativeSwap {
            offer_denom: "uusd".to_string(),
            ask_denom: "uluna".to_string(),
        },
        to: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(create_swap_msg(
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000u128),
            },
            "uluna".to_string()
        ))],
    );

    // optional to address
    // swap_send
    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::NativeSwap {
            offer_denom: "uusd".to_string(),
            ask_denom: "uluna".to_string(),
        },
        to: Some(Addr::unchecked("addr0000")),
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(create_swap_send_msg(
            "addr0000".to_string(),
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000u128),
            },
            "uluna".to_string()
        ))],
    );
    deps.querier
        .with_pairs(&[(&"assetuusd".to_string(), &"pair".to_string())]);
    deps.querier.with_token_balances(&[(
        &"asset".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::PrismSwap {
            offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset")),
            ask_asset_info: AssetInfo::Native("uusd".to_string()),
        },
        to: Some(Addr::unchecked("addr0000")),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair".to_string(),
                amount: Uint128::from(1000000u128),
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::Cw20(Addr::unchecked("asset")),
                        amount: Uint128::from(1000000u128),
                    },
                    belief_price: None,
                    max_spread: None,
                    to: Some("addr0000".to_string()),
                })
                .unwrap()
            })
            .unwrap()
        }))]
    );
}

#[test]
fn query_buy_with_routes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        factory: Addr::unchecked("factory"),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "ukrw".to_string(),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Native("ukrw".to_string()),
                ask_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
            },
            SwapOperation::PrismSwap {
                offer_asset_info: AssetInfo::Cw20(Addr::unchecked("asset0000")),
                ask_asset_info: AssetInfo::Native("uluna".to_string()),
            },
        ],
    };

    deps.querier.with_pairs(&[
        (&"asset0000ukrw".to_string(), &"pair0000".to_string()),
        (&"asset0000uluna".to_string(), &"pair0001".to_string()),
    ]);

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128),
        }
    );

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::NativeSwap {
                offer_denom: "uusd".to_string(),
                ask_denom: "ukrw".to_string(),
            },
            SwapOperation::NativeSwap {
                offer_denom: "ukrw".to_string(),
                ask_denom: "uluna".to_string(),
            },
        ],
    };

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128),
        }
    );
}

#[test]
fn assert_minimum_receive_native_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        "addr0000".to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Native("uusd".to_string()),
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: Addr::unchecked("addr0000"),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Native("uusd".to_string()),
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: Addr::unchecked("addr0000"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "assertion failed; minimum receive amount: 1000001, swap amount: 1000000"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_minimum_receive_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"token0000".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000u128))],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Cw20(Addr::unchecked("token0000")),
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: Addr::unchecked("addr0000"),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Cw20(Addr::unchecked("token0000")),
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: Addr::unchecked("addr0000"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "assertion failed; minimum receive amount: 1000001, swap amount: 1000000"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // failure - invalid token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Cw20(Addr::unchecked("te")),
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: Addr::unchecked("addr0000"),
    };
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid input: human address too short")
    );
}
