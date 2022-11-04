use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use cw721::Cw721ReceiveMsg;
pub use cw_controllers::ClaimsResponse;

#[cw_serde]
pub struct InstantiateMsg {
    /// The collection used for membership
    pub collection: String,
    /// admin can only add/remove hooks, not change other parameters
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Unstake your NFT from the DAO
    Unbond { token_id: String },
    /// Change the admin
    UpdateAdmin { admin: Option<String> },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
    /// Receive NFT to stake
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Show the number of tokens currently staked by this address
    #[returns(StakedResponse)]
    Staked { address: String },

    #[returns(cw_controllers::AdminResponse)]
    Admin {},
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight {},
    #[returns(cw4::MemberListResponse)]
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
}

#[cw_serde]
pub struct StakedResponse {
    pub num_tokens: u64,
}
