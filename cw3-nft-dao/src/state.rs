use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// NFT storage vault
pub const VAULT: Item<Addr> = Item::new("vault");
