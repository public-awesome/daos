use cosmwasm_std::{CosmosMsg, Empty};
use cw3::Vote;
use cw3_flex_multisig::state::Executor;
use cw4::MemberChangedHookMsg;
use cw721::Cw721ReceiveMsg;
use cw_utils::{Duration, Expiration, Threshold};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    /// this is the group contract that contains the member list
    pub group_addr: String,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    /// who is able to execute passed proposals
    /// None means that anyone can execute
    pub executor: Option<Executor>,
    /// code_id of NFT vault (usually a cw721-base or sg721-base)
    pub vault_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Propose {
        title: String,
        description: String,
        // TODO: Empty needs to be StargazeMsgWrapper?
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
    /// Handles update hook messages from the group contract
    MemberChangedHook(MemberChangedHookMsg),
    // / Receive NFT
    // ReceiveNft(Cw721ReceiveMsg),
    // TODO: add a SendNFT message that sends from the vault?
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
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
    // / Returns NFT Vault contract
    // Vault {},
    // TODO: Add vault queries?
}

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// pub struct VaultResponse {
//     pub addr: String,
// }
