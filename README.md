# Stargaze DAOs

A Stargaze DAO is made up of two contracts that work together:

- A cw3 contract that handles proposals and voting ([sg-gov](./contracts/sg-gov/README.md))
- A cw4 contract that handles group membership and weights

The treasury resides in the sg-gov contract. Fungible and non-fungible tokens can be sent to this contract.

The group contract is purely for group memberships, such as membership based on NFT ownership.

This architecture enables multiple DAOs to be managed by the same group.

There should only be one sg-gov contract, and multiple cw4 group contracts. For example there could be a cw4 contract based on NFT ownership, and another one based on governance token staking. They can both use sg-gov for governance.

## Attribution

Parts of this code is licensed from Confio GmbH under the Apache 2.0 license. Namely, the cw4-group, and cw3-flex-multisig contracts.

## DISCLAIMER

STARGAZE CONTRACTS ARE PROVIDED “AS IS”, AT YOUR OWN RISK, AND WITHOUT WARRANTIES OF ANY KIND. No developer or entity involved in creating or instantiating Stargaze smart contracts will be liable for any claims or damages whatsoever associated with your use, inability to use, or your interaction with other users of Stargaze, including any direct, indirect, incidental, special, exemplary, punitive or consequential damages, or loss of profits, cryptocurrencies, tokens, or anything else of value. Although Public Awesome, LLC and it's affilliates developed the initial code for Stargaze, it does not own or control the Stargaze network, which is run by a decentralized validator set.
