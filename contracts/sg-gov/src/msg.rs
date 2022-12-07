use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use cw3::Vote;
use cw4::Cw4Contract;
use cw_utils::{Duration, Expiration, Threshold};
use sg_daos::ContractInstantiateMsg;

use crate::state::Executor;

#[cw_serde]
pub enum Group {
    Cw4Instantiate(ContractInstantiateMsg),
    Cw4Address(String),
}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub description: String,
    pub image: String,
    /// this is the code id for the group contract that contains the member list
    pub group: Group,
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
    UpdateMetadata {
        name: String,
        description: String,
        image: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw_utils::ThresholdResponse)]
    Threshold {},
    #[returns(cw3::ProposalResponse)]
    Proposal { proposal_id: u64 },
    #[returns(cw3::ProposalListResponse)]
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(cw3::ProposalListResponse)]
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(cw3::VoteResponse)]
    Vote { proposal_id: u64, voter: String },
    #[returns(cw3::VoteListResponse)]
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw3::VoterResponse)]
    Voter { address: String },
    #[returns(cw3::VoterListResponse)]
    ListVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(GroupResponse)]
    Group {},
    #[returns(MetadataResponse)]
    Metadata {},
}

#[cw_serde]
pub struct GroupResponse {
    pub group: Cw4Contract,
}

#[cw_serde]
pub struct MetadataResponse {
    pub name: String,
    pub description: String,
    pub image: String,
}
