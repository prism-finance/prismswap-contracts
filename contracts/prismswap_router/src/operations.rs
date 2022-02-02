use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult};

use crate::state::{Config, CONFIG};

use prismswap::asset::{Asset, AssetInfo, PairInfo, PrismSwapAsset};
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
            let amount = query_balance(
                &deps.querier,
                &env.contract.address,
                offer_denom.to_string(),
            )?;

            if let Some(to) = to {
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
                &prismswap_factory,
                &[offer_asset_info.clone(), ask_asset_info],
            )?;

            let amount = match offer_asset_info.clone() {
                AssetInfo::Native(denom) => {
                    query_balance(&deps.querier, &env.contract.address, denom)?
                }
                AssetInfo::Cw20(contract_addr) => {
                    query_token_balance(&deps.querier, &contract_addr, &env.contract.address)?
                }
            };
            let offer_asset: Asset = Asset {
                info: offer_asset_info,
                amount,
            };

            vec![offer_asset.into_swap_msg(&pair_info.contract_addr, None, to)?]
        }
    };

    Ok(Response::new().add_messages(messages))
}
