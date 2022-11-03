# Stargaze DAOs

A Stargaze DAO is made up of two contracts that work together:

- A cw3 contract that handles proposals and voting (sg-voting)
- A cw4 contract that handles group membership and weights

The treasury resides in the cw3 contract. Fungible and non-fungible tokens can be sent to this contract.

The group contract is purely for group memberships, such as membership based on NFT staking. Funds _should not_ be sent to this contract.

This architecture enables multiple DAOs to be backed by the same group.

## Contracts

### [sg-dao](./contracts/sg-dao/README.md)

A cw3 DAO contract that instantiates and wraps a cw4 group. It handles all the voting logic of a DAO and delegates the membership logic to the cw4 group.

It is basically a fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) that integrates the instantiation of the group.

# TODO

## sg-nft-stake

A contract that does membership based on NFT "staking". Anyone can stake an NFT to the contract to become a member. 1 NFT = 1 vote.

# Deprecated

### [sg-nft-group](./contracts/sg-nft-group/README.md)

A cw4 group based on ownership of NFTs in a collection. It can be used as a drop-in replacement for cw4-group in sg-dao to create a DAO based on NFT ownership.

Issue: This iterates through the entire collection, which probably won't work.
