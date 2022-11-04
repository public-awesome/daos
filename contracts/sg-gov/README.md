# Stargaze Governance Contract (DAO)

A cw3 contract that instantiates and wraps a cw4 group or uses a provided cw4 group. It handles all the voting logic of a DAO and delegates the membership logic to the cw4 group.

This contract is basically a fork of [cw3-flex-multisig](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw3-flex-multisig) that integrates the instantiation of the group.
