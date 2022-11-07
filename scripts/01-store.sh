curl -s https://api.github.com/repos/public-awesome/daos/releases/latest \
| grep ".*wasm" \
| cut -d : -f 2,3 \
| tr -d \" \
| wget -qi -

starsd config node $NODE
starsd config chain-id $CHAIN_ID
starsd config output json

starsd tx wasm store sg_gov.wasm --from $ADMIN \
    --admin $ADMIN \
    --gas-prices 0.025ustars --gas-adjustment 1.7 \
    --gas auto -y -b block -o json | jq '.logs' | grep -A 1 code_id

starsd tx wasm store sg_nft_group.wasm --from $ADMIN \
    --admin $ADMIN \
    --gas-prices 0.025ustars --gas-adjustment 1.7 \
    --gas auto -y -b block -o json | jq '.logs' | grep -A 1 code_id
