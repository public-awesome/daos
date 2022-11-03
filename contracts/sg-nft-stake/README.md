# Stargaze NFT Stake Contract

This is an implementation of [cw4 spec](../../packages/cw4/README.md).

It provides a similar API to [`cw4-group`] (which handles elected membership), but rather than appointing members (by admin or multisig), their
membership and weight are based on the number of NFTs from a collection they have staked.