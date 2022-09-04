#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, BlockInfo, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw3::{
    ProposalListResponse, ProposalResponse, VoteInfo, VoteListResponse, VoteResponse, VoterDetail,
    VoterListResponse, VoterResponse,
};
use cw3_fixed_multisig::state::{Proposal, BALLOTS, PROPOSALS};
use cw3_flex_multisig::contract::{
    execute_close, execute_execute, execute_membership_hook, execute_propose, execute_vote,
};
use cw3_flex_multisig::state::{Config, CONFIG};
use cw3_flex_multisig::ContractError as Cw3FlexMultisigError;
use cw4::{Cw4Contract, MemberChangedHookMsg};
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, ThresholdResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:cw3-nft-dao";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        Cw3FlexMultisigError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);
    let total_weight = group_addr.total_weight(&deps.querier)?;
    msg.threshold.validate(total_weight)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        group_addr,
        executor: msg.executor,
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest,
        } => Ok(execute_propose(
            deps,
            env,
            info,
            title,
            description,
            msgs,
            latest,
        )?),
        ExecuteMsg::Vote { proposal_id, vote } => {
            Ok(execute_vote(deps, env, info, proposal_id, vote)?)
        }
        ExecuteMsg::Execute { proposal_id } => Ok(execute_execute(deps, env, info, proposal_id)?),
        ExecuteMsg::Close { proposal_id } => Ok(execute_close(deps, env, info, proposal_id)?),
        ExecuteMsg::MemberChangedHook(MemberChangedHookMsg { diffs }) => {
            Ok(execute_membership_hook(deps, env, info, diffs)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, env, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&list_proposals(deps, env, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&reverse_proposals(deps, env, start_before, limit)?),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::ListVoters { start_after, limit } => {
            to_binary(&list_voters(deps, start_after, limit)?)
        }
    }
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_weight = cfg.group_addr.total_weight(&deps.querier)?;
    Ok(cfg.threshold.to_response(total_weight))
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id)?;
    let status = prop.current_status(&env.block);
    let threshold = prop.threshold.to_response(prop.total_weight);
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        msgs: prop.msgs,
        status,
        expires: prop.expires,
        threshold,
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let proposals = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect::<StdResult<_>>()?;

    Ok(ProposalListResponse { proposals })
}

fn reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_before.map(Bound::exclusive);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(u64, Proposal)>,
) -> StdResult<ProposalResponse> {
    item.map(|(id, prop)| {
        let status = prop.current_status(block);
        let threshold = prop.threshold.to_response(prop.total_weight);
        ProposalResponse {
            id,
            title: prop.title,
            description: prop.description,
            msgs: prop.msgs,
            status,
            expires: prop.expires,
            threshold,
        }
    })
}

fn query_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<VoteResponse> {
    let voter_addr = deps.api.addr_validate(&voter)?;
    let prop = BALLOTS.may_load(deps.storage, (proposal_id, &voter_addr))?;
    let vote = prop.map(|b| VoteInfo {
        proposal_id,
        voter,
        vote: b.vote,
        weight: b.weight,
    });
    Ok(VoteResponse { vote })
}

fn list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoteListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let votes = BALLOTS
        .prefix(proposal_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, ballot)| VoteInfo {
                proposal_id,
                voter: addr.into(),
                vote: ballot.vote,
                weight: ballot.weight,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(VoteListResponse { votes })
}

fn query_voter(deps: Deps, voter: String) -> StdResult<VoterResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voter_addr = deps.api.addr_validate(&voter)?;
    let weight = cfg.group_addr.is_member(&deps.querier, &voter_addr, None)?;

    Ok(VoterResponse { weight })
}

fn list_voters(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let voters = cfg
        .group_addr
        .list_members(&deps.querier, start_after, limit)?
        .into_iter()
        .map(|member| VoterDetail {
            addr: member.addr,
            weight: member.weight,
        })
        .collect();
    Ok(VoterListResponse { voters })
}
