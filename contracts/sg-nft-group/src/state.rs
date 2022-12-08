use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw4::TOTAL_KEY;
use cw_storage_plus::{Item, SnapshotMap, Strategy};

#[cw_serde]
pub struct Config {
    /// The collection that represents this group
    pub collection: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const TOTAL: Item<u64> = Item::new(TOTAL_KEY);

pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    cw4::MEMBERS_KEY,
    cw4::MEMBERS_CHECKPOINTS,
    cw4::MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);

/// Internal collection to store membership NFTs
pub const MEMBER_COLLECTION: Item<Addr> = Item::new("collection");
