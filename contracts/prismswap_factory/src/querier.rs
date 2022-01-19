use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use prismswap::asset::PairInfo;
use prismswap::pair::QueryMsg;

/// ## Description
/// Returns information about the pair described in the structure [`PairInfo`] according to the specified parameters in the `pair_contract` variable.
/// ## Params
/// `pair_contract` it is the type of [`Addr`].
pub fn query_pair_info(querier: &QuerierWrapper, pair_contract: &Addr) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&QueryMsg::Pair {})?,
    }))
}
