use std::str::FromStr;

use crate::asset::PairInfo;
use cosmwasm_std::{Addr, Decimal};
use cw_asset::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TOTAL_FEE: &str = "0.003";
pub const MAX_TOTAL_FEE: &str = "0.05";
pub const DEFAULT_PROTOCOL_FEE: &str = "0.5";
pub const MAX_PROTOCOL_FEE: &str = "0.8";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// ## Description
/// This structure describes a configuration of pair.
pub struct FeeConfig {
    pub total_fee: Decimal,
    pub protocol_fee: Decimal,
}

impl FeeConfig {
    pub fn is_valid(&self) -> bool {
        self.total_fee <= Decimal::from_str(MAX_TOTAL_FEE).unwrap()
            && self.protocol_fee <= Decimal::from_str(MAX_PROTOCOL_FEE).unwrap()
    }
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            total_fee: Decimal::from_str(DEFAULT_TOTAL_FEE).unwrap(),
            protocol_fee: Decimal::from_str(DEFAULT_PROTOCOL_FEE).unwrap(),
        }
    }
}

/// ## Description
/// This structure describes the basic settings for creating a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// CW20 token contract code identifier
    pub token_code_id: u64,
    /// Pair contract code identifier
    pub pair_code_id: u64,
    /// contract address to send fees to
    pub collector: Addr,
    /// address allowed to create pairs and update configuration
    pub owner: Addr,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// UpdateConfig updates relevant code IDs
    UpdateConfig {
        /// CW20 token contract code identifier
        token_code_id: Option<u64>,
        /// Pair contract code identifier
        pair_code_id: Option<u64>,
        /// contract address to send fees to
        collector: Option<Addr>,
        /// address allowed to create pairs and update configuration
        owner: Option<Addr>,
    },
    /// UpdatePairConfig updates configs of pair
    UpdatePairConfig {
        /// assets that indentify the registered pair
        asset_infos: [AssetInfo; 2],
        /// new [`FeeConfig`] settings for pair
        fee_config: FeeConfig,
    },
    /// CreatePair instantiates pair contract
    CreatePair {
        /// the type of asset infos available in [`AssetInfo`]
        asset_infos: [AssetInfo; 2],
        /// [`FeeConfig`] settings for pair, default fees if empty
        fee_config: Option<FeeConfig>,
    },
    /// Deregister removes a previously created pair
    Deregister {
        /// the type of asset infos available in [`AssetInfo`]
        asset_infos: [AssetInfo; 2],
    },
}

/// ## Description
/// This structure describes the query messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns controls settings that specified in custom [`ConfigResponse`] structure
    Config {},
    /// Pair returns a pair according to the specified parameters in `asset_infos` variable.
    Pair {
        /// the type of asset infos available in [`AssetInfo`]
        asset_infos: [AssetInfo; 2],
    },
    /// PairConfig returns a pair info and fee infor according to the specified parameters in `asset_infos` variable.
    PairConfig {
        /// the type of asset infos available in [`AssetInfo`]
        asset_infos: [AssetInfo; 2],
    },
    /// Pairs returns an array of pairs with their configuration according to the specified parameters in `start_after` and `limit` variables.
    Pairs {
        /// the item to start reading from. It is an [`Option`] type that accepts two [`AssetInfo`] elements.
        start_after: Option<[AssetInfo; 2]>,
        /// the number of items to be read. It is an [`Option`] type.
        limit: Option<u32>,
    },
    /// PairsConfig returns an array of pairs with their fee info according to the specified parameters in `start_after` and `limit` variables.
    PairsConfig {
        /// the item to start reading from. It is an [`Option`] type that accepts two [`AssetInfo`] elements.
        start_after: Option<[AssetInfo; 2]>,
        /// the number of items to be read. It is an [`Option`] type.
        limit: Option<u32>,
    },
    /// FeeInfo returns settings that specified in custom [`FeeInfoResponse`] structure
    FeeInfo {
        /// the type of asset infos available in [`AssetInfo`]
        asset_infos: [AssetInfo; 2],
    },
}

/// ## Description
/// A custom struct for each query response that returns controls settings of contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Contract address that used for controls settings for factory, pools and tokenomics contracts
    pub owner: Addr,
    /// CW20 token contract code identifier
    pub token_code_id: u64,
    /// Pair contract code identifier
    pub pair_code_id: u64,
    /// Contract address to send fees to
    pub collector: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairConfigResponse {
    pub pair_info: PairInfo,
    pub fee_config: FeeConfig,
}

/// ## Description
/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// ## Description
/// A custom struct for each query response that returns an array of objects type [`PairInfo`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsConfigResponse {
    pub pairs: Vec<PairConfigResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeInfoResponse {
    pub fee_config: FeeConfig,
    pub collector: Addr,
}
