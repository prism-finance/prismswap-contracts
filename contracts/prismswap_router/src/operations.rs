use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, WasmMsg,
};

use crate::state::{Config, CONFIG};

use cw20::Cw20ExecuteMsg;
use prismswap::asset::{Asset, AssetInfo, PairInfo};
use prismswap::pair::ExecuteMsg as PairExecuteMsg;
use prismswap::querier::{query_balance, query_pair_info, query_token_balance};
use prismswap::router::SwapOperation;
use terra_cosmwasm::{create_swap_msg, create_swap_send_msg, TerraMsgWrapper};

/// Execute swap operation
/// swap all offer asset to ask asset
pub fn execute_swap_operation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
    to: Option<String>,
) -> StdResult<Response<TerraMsgWrapper>> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    let messages: Vec<CosmosMsg<TerraMsgWrapper>> = match operation {
        SwapOperation::NativeSwap {
            offer_denom,
            ask_denom,
        } => {
            let amount =
                query_balance(&deps.querier, env.contract.address, offer_denom.to_string())?;
            if let Some(to) = to {
                // if the operation is last, and requires send
                // deduct tax from the offer_coin
                let asset = Asset {
                    info: AssetInfo::NativeToken {
                        denom: offer_denom.clone(),
                    },
                    amount,
                };
                let amount = amount.checked_sub(asset.compute_tax(&deps.querier)?)?;
                vec![create_swap_send_msg(
                    to,
                    Coin {
                        denom: offer_denom,
                        amount,
                    },
                    ask_denom,
                )]
            } else {
                vec![create_swap_msg(
                    Coin {
                        denom: offer_denom,
                        amount,
                    },
                    ask_denom,
                )]
            }
        }
        SwapOperation::PrismSwap {
            offer_asset_info,
            ask_asset_info,
        } => {
            let config: Config = CONFIG.load(deps.as_ref().storage)?;
            let prismswap_factory = config.factory;
            let pair_info: PairInfo = query_pair_info(
                &deps.querier,
                prismswap_factory,
                &[offer_asset_info.clone(), ask_asset_info],
            )?;

            let amount = match offer_asset_info.clone() {
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address, denom)?
                }
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    deps.api.addr_validate(contract_addr.as_str())?,
                    env.contract.address,
                )?,
            };
            let offer_asset: Asset = Asset {
                info: offer_asset_info,
                amount,
            };

            vec![asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked(pair_info.contract_addr),
                offer_asset,
                None,
                to,
            )?]
        }
    };

    Ok(Response::new().add_messages(messages))
}

pub fn asset_into_swap_msg(
    deps: Deps,
    pair_contract: Addr,
    offer_asset: Asset,
    max_spread: Option<Decimal>,
    to: Option<String>,
) -> StdResult<CosmosMsg<TerraMsgWrapper>> {
    match offer_asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
            // deduct tax first
            let amount = offer_asset
                .amount
                .checked_sub(offer_asset.compute_tax(&deps.querier)?)?;

            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_contract.to_string(),
                funds: vec![Coin { denom, amount }],
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        amount,
                        ..offer_asset
                    },
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            }))
        }
        AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: pair_contract.to_string(),
                amount: offer_asset.amount,
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset,
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            })?,
        })),
    }
}
