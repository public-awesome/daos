use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, QuerierWrapper, Storage};
use cw4::Cw4Contract;
use cw_storage_plus::Item;
use cw_utils::{Duration, Threshold};

use crate::ContractError;

/// Defines who is able to execute proposals once passed
#[cw_serde]
pub enum Executor {
    /// Any member of the voting group, even with 0 points
    Member,
    /// Only the given address
    Only(Addr),
}

#[cw_serde]
pub struct Config {
    pub name: String,
    pub description: String,
    pub image: String,

    pub threshold: Threshold,
    pub max_voting_period: Duration,
    // who is able to execute passed proposals
    // None means that anyone can execute
    pub executor: Option<Executor>,
}

impl Config {
    // Executor can be set in 3 ways:
    // - Member: any member of the voting group is authorized
    // - Only: only passed address is authorized
    // - None: Everyone are authorized
    pub fn authorize(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        sender: &Addr,
    ) -> Result<(), ContractError> {
        if let Some(executor) = &self.executor {
            let group = GROUP.load(storage)?;
            match executor {
                Executor::Member => {
                    group
                        .is_member(querier, sender, None)?
                        .ok_or(ContractError::Unauthorized {})?;
                }
                Executor::Only(addr) => {
                    if addr != sender {
                        return Err(ContractError::Unauthorized {});
                    }
                }
            }
        }
        Ok(())
    }
}

/// Unique items
pub const CONFIG: Item<Config> = Item::new("config");

/// The group that holds DAO members
/// Total weight and voters are queried from this contract
pub const GROUP: Item<Cw4Contract> = Item::new("group");
