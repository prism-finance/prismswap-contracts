use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use prismswap::asset::PairInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// the type of pair info available in [`PairInfo`]
    pub pair_info: PairInfo,
    /// the factory contract address
    pub factory: Addr,
}
