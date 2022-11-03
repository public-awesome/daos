# Stargaze DAOs

A Stargaze DAO is made up of two contracts that work together:

- A cw3 contract that handles proposals and voting ([sg-gov](./contracts/sg-gov/README.md)
- A cw4 contract that handles group membership and weights

The treasury resides in the sg-voting contract. Fungible and non-fungible tokens can be sent to this contract.

The group contract is purely for group memberships, such as membership based on NFT staking. Funds _should not_ be sent to this contract.

This architecture enables multiple DAOs to be backed by the same group.

There should only be one sg-gov contract, and multiple cw4 group contracts. For example there could be a cw4 contract based on NFT staking, and another one based on governance token staking. They can both use sg-voting for governance.
