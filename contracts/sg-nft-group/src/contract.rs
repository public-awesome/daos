#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult,
    Storage, SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw4::{Member, MemberListResponse, MemberResponse, TotalWeightResponse};
use cw721::Cw721ReceiveMsg;
use cw721_base::{ExecuteMsg as Cw721BaseExecuteMsg, MintMsg as Cw721BaseMintMsg};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, ADMIN, COLLECTION, CONFIG, MEMBERS, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-nft-group";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Instantiate a group for the specified collection
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, msg.admin)?)?;

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
    let api = deps.api;
    match msg {
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
        ExecuteMsg::Remove { token_id } => execute_remove(deps, env, info, token_id),
        ExecuteMsg::UpdateAdmin { admin } => {
            Ok(ADMIN.execute_update_admin(deps, info, maybe_addr(api, admin)?)?)
        }
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

    let sender = deps.api.addr_validate(&wrapper.sender)?;
    let height = env.block.height;

    add_member_weight(deps.storage, sender, height)?;
    let join_msg = join(deps.storage, &wrapper.token_id, &wrapper.sender)?;

    Ok(Response::new()
        .add_attribute("action", "receive_nft")
        .add_submessage(join_msg)
        .add_attribute("from", wrapper.sender)
        .add_attribute("token_id", wrapper.token_id))
}

pub fn execute_remove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let member = info.sender;
    // TODO: verify member is the owner of the NFT

    remove_member_weight(deps.storage, member.clone(), env.block.height)?;
    let leave_msgs = leave(deps.storage, &token_id, member.as_ref())?;

    Ok(Response::new()
        .add_submessages(leave_msgs)
        .add_attribute("action", "exit")
        .add_attribute("sender", member))
}

fn add_member_weight(store: &mut dyn Storage, member: Addr, height: u64) -> StdResult<()> {
    MEMBERS.update(store, &member, height, |old| -> StdResult<_> {
        Ok(old.unwrap_or_default() + 1)
    })?;
    TOTAL.update(store, |old| -> StdResult<_> { Ok(old + 1) })?;

    Ok(())
}

fn remove_member_weight(store: &mut dyn Storage, member: Addr, height: u64) -> StdResult<()> {
    MEMBERS.update(store, &member, height, |old| -> StdResult<_> {
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
fn leave(store: &dyn Storage, token_id: &str, owner: &str) -> StdResult<Vec<SubMsg>> {
    let collection = COLLECTION.load(store)?;

    let msg = Cw721BaseExecuteMsg::Burn::<Empty, Empty> {
        token_id: token_id.to_string(),
    };
    let burn_msg = WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };

    let msg = Cw721BaseExecuteMsg::TransferNft::<Empty, Empty> {
        recipient: owner.to_string(),
        token_id: token_id.to_string(),
    };
    let transfer_msg = WasmMsg::Execute {
        contract_addr: CONFIG.load(store)?.collection.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };

    Ok(vec![SubMsg::new(burn_msg), SubMsg::new(transfer_msg)])
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
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
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
    use cosmwasm_std::{
        coin, from_slice, CosmosMsg, OverflowError, OverflowOperation, StdError, Storage,
    };
    use cw4::{member_key, TOTAL_KEY};
    use cw_controllers::{AdminError, HookError};
    use cw_utils::Duration;

    use crate::error::ContractError;

    use super::*;

    const INIT_ADMIN: &str = "juan";
    const USER1: &str = "somebody";
    const USER2: &str = "else";
    const USER3: &str = "funny";
    const CW721_ADDRESS: &str = "wasm1234567890";

    fn default_instantiate(deps: DepsMut) {
        do_instantiate(deps, CW721_ADDRESS)
    }

    fn do_instantiate(deps: DepsMut, collection: &str) {
        let msg = InstantiateMsg {
            collection: collection.to_string(),
            admin: Some(INIT_ADMIN.into()),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    // fn unbond(mut deps: DepsMut, user1: u128, user2: u128, user3: u128, height_delta: u64) {
    //     let mut env = mock_env();
    //     env.block.height += height_delta;

    //     for (addr, stake) in &[(USER1, user1), (USER2, user2), (USER3, user3)] {
    //         if *stake != 0 {
    //             let msg = ExecuteMsg::Exit {
    //                 tokens: Uint128::new(*stake),
    //             };
    //             let info = mock_info(addr, &[]);
    //             execute(deps.branch(), env.clone(), info, msg).unwrap();
    //         }
    //     }
    // }

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        // it worked, let's query the state
        let res = ADMIN.query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(INIT_ADMIN.into()), res.admin);

        let res = query_total_weight(deps.as_ref()).unwrap();
        assert_eq!(0, res.weight);
    }

    fn get_member(deps: Deps, addr: String, at_height: Option<u64>) -> Option<u64> {
        let raw = query(deps, mock_env(), QueryMsg::Member { addr, at_height }).unwrap();
        let res: MemberResponse = from_slice(&raw).unwrap();
        res.weight
    }

    // this tests the member queries
    fn assert_users(
        deps: Deps,
        user1_weight: Option<u64>,
        user2_weight: Option<u64>,
        user3_weight: Option<u64>,
        height: Option<u64>,
    ) {
        let member1 = get_member(deps, USER1.into(), height);
        assert_eq!(member1, user1_weight);

        let member2 = get_member(deps, USER2.into(), height);
        assert_eq!(member2, user2_weight);

        let member3 = get_member(deps, USER3.into(), height);
        assert_eq!(member3, user3_weight);

        // this is only valid if we are not doing a historical query
        if height.is_none() {
            // compute expected metrics
            let weights = vec![user1_weight, user2_weight, user3_weight];
            let sum: u64 = weights.iter().map(|x| x.unwrap_or_default()).sum();
            let count = weights.iter().filter(|x| x.is_some()).count();

            // TODO: more detailed compare?
            let msg = QueryMsg::ListMembers {
                start_after: None,
                limit: None,
            };
            let raw = query(deps, mock_env(), msg).unwrap();
            let members: MemberListResponse = from_slice(&raw).unwrap();
            assert_eq!(count, members.members.len());

            let raw = query(deps, mock_env(), QueryMsg::TotalWeight {}).unwrap();
            let total: TotalWeightResponse = from_slice(&raw).unwrap();
            assert_eq!(sum, total.weight); // 17 - 11 + 15 = 21
        }
    }

    // #[test]
    // fn bond_stake_adds_membership() {
    //     let mut deps = mock_dependencies();
    //     default_instantiate(deps.as_mut());
    //     let height = mock_env().block.height;

    //     // Assert original weights
    //     assert_users(deps.as_ref(), None, None, None, None);

    //     // ensure it rounds down, and respects cut-off
    //     bond(deps.as_mut(), 12_000, 7_500, 4_000, 1);

    //     // Assert updated weights
    //     assert_stake(deps.as_ref(), 12_000, 7_500, 4_000);
    //     assert_users(deps.as_ref(), Some(12), Some(7), None, None);

    //     // add some more, ensure the sum is properly respected (7.5 + 7.6 = 15 not 14)
    //     bond(deps.as_mut(), 0, 7_600, 1_200, 2);

    //     // Assert updated weights
    //     assert_stake(deps.as_ref(), 12_000, 15_100, 5_200);
    //     assert_users(deps.as_ref(), Some(12), Some(15), Some(5), None);

    //     // check historical queries all work
    //     assert_users(deps.as_ref(), None, None, None, Some(height + 1)); // before first stake
    //     assert_users(deps.as_ref(), Some(12), Some(7), None, Some(height + 2)); // after first stake
    //     assert_users(deps.as_ref(), Some(12), Some(15), Some(5), Some(height + 3));
    //     // after second stake
    // }

    // #[test]
    // fn unbond_stake_update_membership() {
    //     let mut deps = mock_dependencies();
    //     default_instantiate(deps.as_mut());
    //     let height = mock_env().block.height;

    //     // ensure it rounds down, and respects cut-off
    //     bond(deps.as_mut(), 12_000, 7_500, 4_000, 1);
    //     unbond(deps.as_mut(), 4_500, 2_600, 1_111, 2);

    //     // Assert updated weights
    //     assert_stake(deps.as_ref(), 7_500, 4_900, 2_889);
    //     assert_users(deps.as_ref(), Some(7), None, None, None);

    //     // Adding a little more returns weight
    //     bond(deps.as_mut(), 600, 100, 2_222, 3);

    //     // Assert updated weights
    //     assert_users(deps.as_ref(), Some(8), Some(5), Some(5), None);

    //     // check historical queries all work
    //     assert_users(deps.as_ref(), None, None, None, Some(height + 1)); // before first stake
    //     assert_users(deps.as_ref(), Some(12), Some(7), None, Some(height + 2)); // after first bond
    //     assert_users(deps.as_ref(), Some(7), None, None, Some(height + 3)); // after first unbond
    //     assert_users(deps.as_ref(), Some(8), Some(5), Some(5), Some(height + 4)); // after second bond

    //     // error if try to unbond more than stake (USER2 has 5000 staked)
    //     let msg = ExecuteMsg::Exit {
    //         tokens: Uint128::new(5100),
    //     };
    //     let mut env = mock_env();
    //     env.block.height += 5;
    //     let info = mock_info(USER2, &[]);
    //     let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    //     assert_eq!(
    //         err,
    //         ContractError::Std(StdError::overflow(OverflowError::new(
    //             OverflowOperation::Sub,
    //             5000,
    //             5100
    //         )))
    //     );
    // }

    // #[test]
    // fn raw_queries_work() {
    //     // add will over-write and remove have no effect
    //     let mut deps = mock_dependencies();
    //     default_instantiate(deps.as_mut());
    //     // Set values as (11, 6, None)
    //     bond(deps.as_mut(), 11_000, 6_000, 0, 1);

    //     // get total from raw key
    //     let total_raw = deps.storage.get(TOTAL_KEY.as_bytes()).unwrap();
    //     let total: u64 = from_slice(&total_raw).unwrap();
    //     assert_eq!(17, total);

    //     // get member votes from raw key
    //     let member2_raw = deps.storage.get(&member_key(USER2)).unwrap();
    //     let member2: u64 = from_slice(&member2_raw).unwrap();
    //     assert_eq!(6, member2);

    //     // and execute misses
    //     let member3_raw = deps.storage.get(&member_key(USER3));
    //     assert_eq!(None, member3_raw);
    // }
}
