use std::marker::PhantomData;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult,
    Storage, SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw4::{Member, MemberListResponse, MemberResponse, TotalWeightResponse};
use cw721::Cw721ReceiveMsg;
use cw721_base::helpers::Cw721Contract;
use cw721_base::{ExecuteMsg as Cw721BaseExecuteMsg, MintMsg as Cw721BaseMintMsg};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, COLLECTION, CONFIG, MEMBERS, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-nft-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Instantiate a group for the specified collection
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let api = deps.api;

    let config = Config {
        collection: api.addr_validate(&msg.collection)?,
    };
    CONFIG.save(deps.storage, &config)?;
    TOTAL.save(deps.storage, &0)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
        ExecuteMsg::Remove { token_id } => execute_remove(deps, env, info, token_id),
    }
}

pub fn execute_receive_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let collection = CONFIG.load(deps.storage)?.collection;

    if info.sender != collection {
        return Err(ContractError::InvalidCollection {
            received: info.sender,
            expected: collection,
        });
    }

    let Cw721ReceiveMsg {
        sender, token_id, ..
    } = wrapper;

    add_member_weight(
        deps.storage,
        &deps.api.addr_validate(&sender)?,
        env.block.height,
    )?;

    Ok(Response::new()
        .add_attribute("action", "receive_nft")
        .add_submessage(join(deps.storage, &token_id, &sender)?)
        .add_attribute("from", sender)
        .add_attribute("token_id", token_id))
}

pub fn execute_remove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let member = info.sender;

    only_owner(
        deps.as_ref(),
        &member,
        &COLLECTION.load(deps.storage)?,
        &token_id,
    )?;

    remove_member_weight(deps.storage, &member, env.block.height)?;

    Ok(Response::new()
        .add_submessages(leave(deps.storage, &token_id, member.as_ref())?)
        .add_attribute("action", "exit")
        .add_attribute("sender", member))
}

fn only_owner(
    deps: Deps,
    sender: &Addr,
    collection: &Addr,
    token_id: &str,
) -> Result<String, ContractError> {
    Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData)
        .owner_of(&deps.querier, token_id, false)
        .map(|res| {
            if res.owner != *sender {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(res.owner)
            }
        })?
}

fn add_member_weight(store: &mut dyn Storage, member: &Addr, height: u64) -> StdResult<()> {
    MEMBERS.update(store, member, height, |old| -> StdResult<_> {
        Ok(old.unwrap_or_default() + 1)
    })?;
    TOTAL.update(store, |old| -> StdResult<_> { Ok(old + 1) })?;

    Ok(())
}

fn remove_member_weight(store: &mut dyn Storage, member: &Addr, height: u64) -> StdResult<()> {
    MEMBERS.update(store, member, height, |old| -> StdResult<_> {
        Ok(old.unwrap_or_default() - 1)
    })?;
    TOTAL.update(store, |old| -> StdResult<_> { Ok(old - 1) })?;

    Ok(())
}

/// To the join the group, the sent NFT is minted into the internal collection.
fn join(store: &dyn Storage, token_id: &str, owner: &str) -> StdResult<SubMsg> {
    let mint_msg = Cw721BaseMintMsg::<Empty> {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: None,
        extension: Empty {},
    };
    let msg = Cw721BaseExecuteMsg::Mint::<Empty, Empty>(mint_msg);

    let msg = WasmMsg::Execute {
        contract_addr: COLLECTION.load(store)?.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };

    Ok(SubMsg::new(msg))
}

/// To leave the group, we have to burn the NFT from the internal collection.
/// Then we have to transfer it from the collection to the original owner.
fn leave(store: &dyn Storage, token_id: &str, member: &str) -> StdResult<Vec<SubMsg>> {
    let transfer_msg = WasmMsg::Execute {
        contract_addr: CONFIG.load(store)?.collection.to_string(),
        msg: to_binary(&Cw721BaseExecuteMsg::TransferNft::<Empty, Empty> {
            recipient: member.to_string(),
            token_id: token_id.to_string(),
        })?,
        funds: vec![],
    };

    let burn_msg = WasmMsg::Execute {
        contract_addr: COLLECTION.load(store)?.to_string(),
        msg: to_binary(&Cw721BaseExecuteMsg::Burn::<Empty, Empty> {
            token_id: token_id.to_string(),
        })?,
        funds: vec![],
    };

    Ok(vec![SubMsg::new(transfer_msg), SubMsg::new(burn_msg)])
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
        QueryMsg::Collection {} => to_binary(&query_collection(deps)?),
    }
}

fn query_total_weight(deps: Deps) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

pub fn query_collection(deps: Deps) -> StdResult<String> {
    Ok(CONFIG.load(deps.storage)?.collection.to_string())
}

fn query_member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<MemberResponse> {
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    const CW721_ADDRESS: &str = "wasm1234567890";

    fn default_instantiate(deps: DepsMut) {
        do_instantiate(deps, CW721_ADDRESS)
    }

    fn do_instantiate(deps: DepsMut, collection: &str) {
        let msg = InstantiateMsg {
            collection: collection.to_string(),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        let res = query_total_weight(deps.as_ref()).unwrap();
        assert_eq!(0, res.weight);
    }
}
