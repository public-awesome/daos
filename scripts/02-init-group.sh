starsd config node $NODE
starsd config chain-id $CHAIN_ID
starsd config output json

MSG=$(cat <<EOF
{
  "collection": "$COLLECTION"
}
EOF
)
 
starsd tx wasm instantiate $SG_NFT_GROUP_CODE_ID "$MSG" --label "NFT-Group" \
    --admin $ADMIN \
    --gas-prices 0.025ustars --gas auto --gas-adjustment 1.9 \
    --from $ADMIN -y -b block -o json | jq .
