use crate::asset::PairInfo;
use crate::factory::{
    ConfigResponse as FactoryConfigResponse, FeeInfoResponse, PairsResponse,
    QueryMsg as FactoryQueryMsg,
};
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};

use cosmwasm_std::{
    to_binary, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Coin, QuerierWrapper,
    QueryRequest, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use cw_asset::{Asset, AssetInfo};

/// ## Description
/// Returns the balance of the denom at the specified account address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **account_addr** is the object of type [`Addr`].
///
/// * **denom** is the object of type [`String`].
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: &Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

/// ## Description
/// Returns the total balance for all coins at the specified account address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **account_addr** is the object of type [`Addr`].
pub fn query_all_balances(querier: &QuerierWrapper, account_addr: &Addr) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: String::from(account_addr),
        }))?;
    Ok(all_balances.amount)
}

/// ## Description
/// Returns the token balance at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`]. Sets the address of the contract for which
/// the balance will be requested
///
/// * **account_addr** is the object of type [`Addr`].
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // load balance from the token contract
    let res: Cw20BalanceResponse = querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&Cw20QueryMsg::Balance {
                address: String::from(account_addr),
            })?,
        }))
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(res.balance)
}

/// ## Description
/// Returns the token symbol at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`].
pub fn query_token_symbol(querier: &QuerierWrapper, contract_addr: &Addr) -> StdResult<String> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.symbol)
}

/// ## Description
/// Returns the total supply at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`].
pub fn query_supply(querier: &QuerierWrapper, contract_addr: &Addr) -> StdResult<Uint128> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// ## Description
/// Returns the config of factory contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **factory_contract** is the object of type [`Addr`].
pub fn query_factory_config(
    querier: &QuerierWrapper,
    factory_contract: &Addr,
) -> StdResult<FactoryConfigResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Config {})?,
    }))
}

/// ## Description
/// Returns the fee configuration for the specified pair.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **factory_contract** is the object of type [`Addr`].
///
/// * **asset_infos** is an array that contains two items of type [`AssetInfo`].
pub fn query_fee_info(
    querier: &QuerierWrapper,
    factory_contract: &Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<FeeInfoResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::FeeInfo {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

/// ## Description
/// Returns the pair information at the specified assets of type [`AssetInfo`].
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **factory_contract** is the object of type [`Addr`].
///
/// * **asset_infos** is an array that contains two items of type [`AssetInfo`].
pub fn query_pair_info(
    querier: &QuerierWrapper,
    factory_contract: &Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

/// ## Description
/// Returns the vector that contains items of type [`PairInfo`]
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **factory_contract** is the object of type [`Addr`].
///
/// * **start_after** is an [`Option`] field that contains array with two items of type [`AssetInfo`].
///
/// * **limit** is an [`Option`] field of type [`u32`].
pub fn query_pairs_info(
    querier: &QuerierWrapper,
    factory_contract: &Addr,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Pairs { start_after, limit })?,
    }))
}

/// ## Description
/// Returns information about the simulation of the swap in a [`SimulationResponse`] object.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **pair_contract** is the object of type [`Addr`].
///
/// * **offer_asset** is the object of type [`Asset`].
pub fn simulate(
    querier: &QuerierWrapper,
    pair_contract: &Addr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        })?,
    }))
}

/// ## Description
/// Returns information about the reverse simulation in a [`ReverseSimulationResponse`] object.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **pair_contract** is the object of type [`Addr`].
///
/// * **ask_asset** is the object of type [`Asset`].
pub fn reverse_simulate(
    querier: &QuerierWrapper,
    pair_contract: &Addr,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        })?,
    }))
}
