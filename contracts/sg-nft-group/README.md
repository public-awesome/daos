# Stargaze NFT Group Contract

A contract that does membership based on NFT ownership in a specific collection. Anyone can send an NFT to the contract to become a member. 1 NFT = 1 vote.

This is an implementation of [cw4 spec](../../packages/cw4/README.md).

It provides a similar API to [`cw4-group`] (which handles elected membership), but rather than appointing members (by admin or multisig), their
membership and weight are based on the number of NFTs from a collection they have added to the group.

_NOTE_: It's important not to confuse NFTs used for membership vs. NFTs in the DAO treasury. The NFTs in this contract are purely for group membership that is used by sg-gov for voting weights.
