use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721::Cw721ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// The collection used for membership
    pub collection: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive NFT to join and/or add voting power to a member
    ReceiveNft(Cw721ReceiveMsg),
    /// Remove NFT to reduce voting power or leave the group
    Remove { token_id: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    Collection {},
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
}
