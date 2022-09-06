#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw4::{
    HooksResponse, Member, MemberDiff, MemberListResponse, MemberResponse, TotalWeightResponse,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, ADMIN, CONFIG, MEMBERS, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-nft-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    create(deps, msg.admin, msg.collection_addr, env.block.height)?;
    Ok(Response::default())
}

// create is the instantiation logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create(
    mut deps: DepsMut,
    admin: Option<String>,
    collection_addr: String,
    height: u64,
) -> Result<(), ContractError> {
    let admin_addr = admin
        .map(|admin| deps.api.addr_validate(&admin))
        .transpose()?;
    ADMIN.set(deps.branch(), admin_addr)?;

    let collection = deps.api.addr_validate(&collection_addr)?;

    CONFIG.save(deps.storage, &Config { collection })?;

    update_members(deps.branch(), height)?;

    Ok(())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => Ok(ADMIN.execute_update_admin(
            deps,
            info,
            admin.map(|admin| api.addr_validate(&admin)).transpose()?,
        )?),
        ExecuteMsg::AddHook { addr: _ } => Err(ContractError::HooksUnsupported {}),
        ExecuteMsg::RemoveHook { addr: _ } => Err(ContractError::HooksUnsupported {}),
        ExecuteMsg::UpdateMembers {} => execute_update_members(deps, env, info),
    }
}

/// Note that anyone can call this.
pub fn execute_update_members(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let attributes = vec![
        attr("action", "update_members"),
        attr("sender", &info.sender),
    ];

    update_members(deps.branch(), env.block.height)?;
    Ok(Response::new().add_attributes(attributes))
}

pub fn update_members(deps: DepsMut, height: u64) -> Result<(), ContractError> {
    let collection = CONFIG.load(deps.storage)?.collection;

    let mut total = 0u64;
    let mut diffs: Vec<MemberDiff> = vec![];

    // TODO: check how much gas this takes for 10,000 tokens
    // TODO: maybe check the number of tokens in collection, and have a max?
    // TODO: check limit

    // fetch all owners from the collection
    let all_tokens_msg = Cw721QueryMsg::AllTokens {
        start_after: None,
        limit: None,
    };
    let res: TokensResponse = deps
        .querier
        .query_wasm_smart(collection.to_string(), &all_tokens_msg)?;

    // create a member for each owner
    for token in res.tokens {
        let res: OwnerOfResponse = deps.querier.query_wasm_smart(
            collection.to_string(),
            &Cw721QueryMsg::OwnerOf {
                token_id: token.clone(),
                include_expired: None,
            },
        )?;
        let owner = res.owner;

        let add_addr = deps.api.addr_validate(&owner)?;
        MEMBERS.update(deps.storage, &add_addr, height, |old| -> StdResult<_> {
            let old_weight = old.unwrap_or_default();
            let new_weight = old_weight + 1;
            total += 1;
            diffs.push(MemberDiff::new(owner, old, Some(new_weight)));
            Ok(new_weight)
        })?;
    }

    TOTAL.save(deps.storage, &total)?;

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member {
            addr,
            at_height: height,
        } => to_binary(&query_member(deps, addr, height)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&list_members(deps, start_after, limit)?)
        }
        QueryMsg::TotalWeight {} => to_binary(&query_total_weight(deps)?),
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::Hooks {} => to_binary(&query_hooks()?),
    }
}

pub fn query_hooks() -> StdResult<HooksResponse> {
    Ok(HooksResponse { hooks: vec![] })
}

pub fn query_total_weight(deps: Deps) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

pub fn query_member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<MemberResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
        None => MEMBERS.may_load(deps.storage, &addr),
    }?;
    Ok(MemberResponse { weight })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let members = MEMBERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, weight)| Member {
                addr: addr.into(),
                weight,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(MemberListResponse { members })
}
