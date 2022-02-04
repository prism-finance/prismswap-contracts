use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_cosmwasm::TerraMsgWrapper;

use crate::pair::ExecuteMsg as PairExecuteMsg;
use crate::querier::{query_balance, query_token_balance};
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
    fn to_string_legacy(&self) -> String;
    fn check(&self, api: &dyn Api) -> StdResult<()>;
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

    fn to_string_legacy(&self) -> String {
        match self {
            AssetInfo::Cw20(contract_addr) => contract_addr.to_string(),
            AssetInfo::Native(denom) => denom.to_string(),
        }
    }

    fn check(&self, api: &dyn Api) -> StdResult<()> {
        if let AssetInfo::Cw20(addr) = self {
            api.addr_validate(addr.as_str())?;
        }
        Ok(())
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
    fn to_string_legacy(&self) -> String;
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
    fn to_string_legacy(&self) -> String {
        format!("{}:{}", self.info.to_string_legacy(), self.amount)
    }
}
