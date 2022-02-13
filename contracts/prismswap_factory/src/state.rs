use prismswap::{
    asset::{AssetInfo, PrismSwapAssetInfo},
    factory::FeeConfig,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub collector: Addr,
    pub pairs_admin: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TmpPairInfo {
    pub pair_key: Vec<u8>,
    pub fee_config: FeeConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairConfig {
    pub pair_address: Addr,
    pub fee_config: FeeConfig,
}

pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");
pub const PAIRS: Map<&[u8], PairConfig> = Map::new("pair_config");

pub fn pair_key(asset_infos: &[AssetInfo; 2]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs(
    storage: &dyn Storage,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<PairConfig>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::exclusive);

    PAIRS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<PairConfig>>>()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<[AssetInfo; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}
