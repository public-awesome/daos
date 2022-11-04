# Stargaze Governance Contract (DAO)

A cw3 contract that instantiates and wraps a cw4 group or uses a provided cw4 group. It handles all the voting logic of a DAO and delegates the membership logic to the cw4 group.

This contract is basically a fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) that integrates the instantiation of the group.

# TODO

- [ ] Take in a group address or code id. If the address is specified, use that for the group. If not, create a new one with the code id. This way, a new DAO can be formed from the same group. Multiple DAOs can be created with the same group.
