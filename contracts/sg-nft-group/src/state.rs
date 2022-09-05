use cosmwasm_std::Addr;
use cw4::TOTAL_KEY;
use cw_controllers::Admin;
use cw_storage_plus::{Item, SnapshotMap, Strategy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ADMIN: Admin = Admin::new("admin");

pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);

pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    cw4::MEMBERS_KEY,
    cw4::MEMBERS_CHECKPOINTS,
    cw4::MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub collection: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
