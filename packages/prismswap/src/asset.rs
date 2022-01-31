use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_cosmwasm::TerraMsgWrapper;

use crate::pair::ExecuteMsg as PairExecuteMsg;
use crate::querier::{query_balance, query_token_balance, query_token_symbol};
use cosmwasm_std::{
    to_binary, Addr, Api, Coin, CosmosMsg, Decimal, MessageInfo, QuerierWrapper, StdError,
    StdResult, Uint128, WasmMsg,
};

pub use cw_asset::{Asset, AssetInfo};

/// ## Description
/// This structure describes the main controls configs of pair
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInfo {
    /// the type of asset infos available in [`AssetInfo`]
    pub asset_infos: [AssetInfo; 2],
    /// pair contract address
    pub contract_addr: Addr,
    /// pair liquidity token
    pub liquidity_token: Addr,
}

impl PairInfo {
    /// ## Description
    /// Returns balance for each asset in the pool.
    /// ## Params
    /// * **self** is the type of the caller object
    ///
    /// * **querier** is the object of type [`QuerierWrapper`]
    ///
    /// * **contract_addr** is the pool address of the pair.
    pub fn query_pools(
        &self,
        querier: &QuerierWrapper,
        contract_addr: &Addr,
    ) -> StdResult<[Asset; 2]> {
        Ok([
            Asset {
                amount: self.asset_infos[0].query_pool(querier, contract_addr)?,
                info: self.asset_infos[0].clone(),
            },
            Asset {
                amount: self.asset_infos[1].query_pool(querier, contract_addr)?,
                info: self.asset_infos[1].clone(),
            },
        ])
    }
}

pub trait PrismSwapAssetInfo {
    fn is_native_token(&self) -> bool;
    fn query_pool(&self, querier: &QuerierWrapper, pool_addr: &Addr) -> StdResult<Uint128>;
    fn as_bytes(&self) -> &[u8];
}

impl PrismSwapAssetInfo for AssetInfo {
    /// ## Description
    /// Returns true if the caller is a native token. Otherwise returns false.
    /// ## Params
    /// * **self** is the type of the caller object
    fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::Cw20(..) => false,
            AssetInfo::Native(..) => true,
        }
    }

    /// ## Description
    /// Returns balance of token in a pool.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **pool_addr** is the address of the contract from which the balance is requested.
    fn query_pool(&self, querier: &QuerierWrapper, pool_addr: &Addr) -> StdResult<Uint128> {
        match self {
            AssetInfo::Cw20(contract_addr) => {
                query_token_balance(querier, contract_addr, pool_addr)
            }
            AssetInfo::Native(denom) => query_balance(querier, pool_addr, denom.to_string()),
        }
    }

    /// ## Description
    /// If caller object is a native token of type ['AssetInfo`] then his `denom` field convert to a byte string.
    ///
    /// If caller object is a token of type ['AssetInfo`] then his `contract_addr` field convert to a byte string.
    /// ## Params
    /// * **self** is the type of the caller object.
    fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfo::Native(denom) => denom.as_bytes(),
            AssetInfo::Cw20(contract_addr) => contract_addr.as_bytes(),
        }
    }
}

pub trait PrismSwapAsset {
    fn into_swap_msg(
        self,
        pair_contract: &Addr,
        max_spread: Option<Decimal>,
        to: Option<String>,
    ) -> StdResult<CosmosMsg<TerraMsgWrapper>>;
    fn assert_sent_native_token_balance(&self, info: &MessageInfo) -> StdResult<()>;
}

impl PrismSwapAsset for Asset {
    fn into_swap_msg(
        self,
        pair_contract: &Addr,
        max_spread: Option<Decimal>,
        to: Option<String>,
    ) -> StdResult<CosmosMsg<TerraMsgWrapper>> {
        match self.info.clone() {
            AssetInfo::Native(denom) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_contract.to_string(),
                funds: vec![Coin {
                    denom,
                    amount: self.amount,
                }],
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        amount: self.amount,
                        info: self.info,
                    },
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            })),
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: pair_contract.to_string(),
                    amount: self.amount,
                    msg: to_binary(&PairExecuteMsg::Swap {
                        offer_asset: self,
                        belief_price: None,
                        max_spread,
                        to,
                    })?,
                })?,
            })),
        }
    }

    fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::Native(denom) = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

/// ## Description
/// Returns the validated address in lowercase on success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is a object of type [`Api`]
///
/// * **addr** is the object of type [`Addr`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(addr)
}

// we need 6 for xPRISM
const TOKEN_SYMBOL_MAX_LENGTH: usize = 6;
pub fn format_lp_token_name(
    asset_infos: &[AssetInfo; 2],
    querier: &QuerierWrapper,
) -> StdResult<String> {
    let mut short_symbols: Vec<String> = vec![];
    for asset_info in asset_infos {
        let short_symbol: String;
        match asset_info {
            AssetInfo::Native(denom) => {
                short_symbol = denom.chars().take(TOKEN_SYMBOL_MAX_LENGTH).collect();
            }
            AssetInfo::Cw20(contract_addr) => {
                let token_symbol = query_token_symbol(querier, contract_addr)?;
                short_symbol = token_symbol.chars().take(TOKEN_SYMBOL_MAX_LENGTH).collect();
            }
        }
        short_symbols.push(short_symbol);
    }
    Ok(format!("{}-{}-LP", short_symbols[0], short_symbols[1]).to_uppercase())
}
