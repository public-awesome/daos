use base_factory::{msg::BaseMinterCreateMsg, state::BaseMinterParams};
use cosmwasm_std::Empty;
use cw_utils::{Duration, Threshold};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg4::MinterConfigResponse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub create_msg: BaseMinterCreateMsg,
    pub params: BaseMinterParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint {
        group_addr: String,
        threshold: Threshold,
        max_voting_period: Duration,
        dao_code_id: u64,
    },
}

pub type ConfigResponse = MinterConfigResponse<Empty>;
