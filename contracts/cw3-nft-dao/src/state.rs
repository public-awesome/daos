use cw3_flex_multisig::state::Executor;
use cw4::Cw4Contract;
use cw_storage_plus::Item;
use cw_utils::{Duration, Threshold};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    // who is able to execute passed proposals
    // None means that anyone can execute
    pub executor: Option<Executor>,
}

// impl Config {
//     // Executor can be set in 3 ways:
//     // - Member: any member of the voting group is authorized
//     // - Only: only passed address is authorized
//     // - None: Everyone are authorized
//     pub fn authorize(&self, querier: &QuerierWrapper, sender: &Addr) -> Result<(), ContractError> {
//         if let Some(executor) = &self.executor {
//             match executor {
//                 Executor::Member => {
//                     self.group_addr
//                         .is_member(querier, sender, None)?
//                         .ok_or(ContractError::Unauthorized {})?;
//                 }
//                 Executor::Only(addr) => {
//                     if addr != sender {
//                         return Err(ContractError::Unauthorized {});
//                     }
//                 }
//             }
//         }
//         Ok(())
//     }
// }

/// Unique items
pub const CONFIG: Item<Config> = Item::new("config");

/// The group that holds DAO members
/// Total weight and voters are queried from this contract
pub const GROUP: Item<Cw4Contract> = Item::new("group");
