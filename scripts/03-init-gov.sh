starsd config node $NODE
starsd config chain-id $CHAIN_ID
starsd config output json

MSG=$(cat <<EOF
{
	"group": {
		"cw4_address": "$SG_NFT_GROUP"
	},
	"threshold": {
		"threshold_quorum": {
			"threshold": "0.66",
			"quorum": "0.40"
		}
	},
	"max_voting_period": {
		"time": 604800
	}
}
EOF
)
 
starsd tx wasm instantiate $SG_GOV_CODE_ID "$MSG" --label "Gov" \
    --admin $ADMIN \
    --gas-prices 0.025ustars --gas auto --gas-adjustment 1.9 \
    --from $ADMIN -y -b block -o json | jq .
