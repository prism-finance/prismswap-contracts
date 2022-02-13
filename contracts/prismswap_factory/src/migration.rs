use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, CONFIG};

pub const LEGACY: Item<LegacyConfig> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyConfig {
    pub owner: Addr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub collector: Addr,
}

pub fn migrate_config(storage: &mut dyn Storage, pairs_admin: Addr) -> StdResult<()> {
    let legacy_config: LegacyConfig = LEGACY.load(storage)?;
    let config = Config {
        owner: legacy_config.owner,
        token_code_id: legacy_config.token_code_id,
        pair_code_id: legacy_config.pair_code_id,
        collector: legacy_config.collector,
        pairs_admin,
    };

    CONFIG.save(storage, &config)?;
    Ok(())
}

#[cfg(test)]
mod migrate_tests {
    use cosmwasm_std::{testing::mock_dependencies, Api};

    use crate::{
        migration::{migrate_config, LegacyConfig, LEGACY},
        state::{Config, CONFIG},
    };

    #[test]
    fn test_config_migration() {
        let mut deps = mock_dependencies(&[]);

        LEGACY
            .save(
                &mut deps.storage,
                &LegacyConfig {
                    owner: deps.api.addr_validate("owner0000").unwrap(),
                    token_code_id: 2,
                    pair_code_id: 33,
                    collector: deps.api.addr_validate("collector0000").unwrap(),
                },
            )
            .unwrap();

        migrate_config(
            &mut deps.storage,
            deps.api.addr_validate("admin0000").unwrap(),
        )
        .unwrap();

        let config: Config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(
            config,
            Config {
                owner: deps.api.addr_validate("owner0000").unwrap(),
                token_code_id: 2,
                pair_code_id: 33,
                collector: deps.api.addr_validate("collector0000").unwrap(),
                pairs_admin: deps.api.addr_validate("admin0000").unwrap(),
            }
        )
    }
}
