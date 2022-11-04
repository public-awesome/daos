use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CosmosMsg, Empty};
use cw3::Vote;
use cw3_flex_multisig::state::Executor;
use cw4::{Cw4Contract, Member};
use cw_utils::{Duration, Expiration, Threshold};

#[cw_serde]
pub struct InstantiateMsg {
    /// this is the code id for the group contract that contains the member list
    pub group_code_id: u64,
    pub members: Vec<Member>,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    /// who is able to execute passed proposals
    /// None means that anyone can execute
    pub executor: Option<Executor>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Propose {
        title: String,
        description: String,
        // TODO: Empty needs to be StargazeMsgWrapper for Stargaze proposals like creating a collection?
        msgs: Vec<CosmosMsg<Empty>>,
        // note: we ignore API-spec'd earliest if passed, always opens immediately
        latest: Option<Expiration>,
    },
    Vote {
        proposal_id: u64,
        vote: Vote,
    },
    Execute {
        proposal_id: u64,
    },
    Close {
        proposal_id: u64,
    },
}

#[cw_serde]
pub enum QueryMsg {
    /// Return ThresholdResponse
    Threshold {},
    /// Returns ProposalResponse
    Proposal { proposal_id: u64 },
    /// Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns ProposalListResponse
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns VoteResponse
    Vote { proposal_id: u64, voter: String },
    /// Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns VoterInfo
    Voter { address: String },
    /// Returns VoterListResponse
    ListVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns GroupResponse
    Group {},
}

#[cw_serde]
pub struct GroupResponse {
    pub group: Cw4Contract,
}
