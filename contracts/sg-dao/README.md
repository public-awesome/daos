# Stargaze DAO

This contract is basically a fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) that integrates the instantiation of the group, and adds membership voting by default.

# TODO

- [ ] Take in a group address or code id. If the address is specified, use that for the group. If not, create a new one with the code id. This way, a new DAO can be formed from the same group. Multiple DAOs can be created with the same group.
