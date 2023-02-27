#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        contract::{execute, instantiate, query, reply, CONTRACT_NAME, CONTRACT_VERSION},
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    };
    use cosmwasm_std::{
        coin, from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Empty, Uint128,
    };
    use cw2::{query_contract_info, ContractVersion};
    use cw4::{Member, MemberListResponse, MemberResponse};
    use cw721_base::{
        msg::{ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg},
        Extension, MintMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use sg_daos::{Admin, ContractInstantiateMsg};

    const OWNER: &str = "admin0001";
    const MEMBER1: &str = "member0001";
    const MEMBER2: &str = "member0002";

    const COLLECTION_CONTRACT: &str = "contract0";
    const SG_NFT_GROUP_CONTRACT: &str = "contract1";
    const MEMBERSHIP_NFT_CONTRACT: &str = "contract2";

    fn member<T: Into<String>>(addr: T, weight: u64) -> Member {
        Member {
            addr: addr.into(),
            weight,
        }
    }

    fn members() -> Vec<Member> {
        vec![member(OWNER, 1), member(MEMBER1, 1), member(MEMBER2, 2)]
    }

    pub fn contract_nft_group() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
        Box::new(contract)
    }

    pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw721_base::entry::execute,
            cw721_base::entry::instantiate,
            cw721_base::entry::query,
        );
        Box::new(contract)
    }

    fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
                .unwrap();
        })
    }

    const MINTER: &str = "minter";

    #[track_caller]
    fn instantiate_collection(app: &mut App) -> Addr {
        let collection_code_id = app.store_code(contract_cw721());
        let msg = Cw721InstantiateMsg {
            name: "My NFTs".to_string(),
            symbol: "NFT".to_string(),
            minter: MINTER.into(),
        };
        app.instantiate_contract(
            collection_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "collection",
            None,
        )
        .unwrap()
    }

    /// create a sg_nft_group initialized with the given members
    fn sg_nft_group_init_info(app: &mut App) -> ContractInstantiateMsg {
        let group_id = app.store_code(contract_nft_group());
        let collection_code_id = app.store_code(contract_cw721());

        let msg = Cw721InstantiateMsg {
            name: "MemberCollection".to_string(),
            symbol: "SGMC".to_string(),
            minter: SG_NFT_GROUP_CONTRACT.to_string(),
        };

        let cw721_init_msg = ContractInstantiateMsg {
            code_id: collection_code_id,
            msg: to_binary(&msg).unwrap(),
            admin: Some(Admin::Creator {}),
            label: "MemberCollection".to_string(),
        };

        let collection = instantiate_collection(app);
        let msg = InstantiateMsg {
            collection: collection.to_string(),
            cw721_init_msg,
        };

        ContractInstantiateMsg {
            code_id: group_id,
            msg: to_binary(&msg).unwrap(),
            admin: Some(Admin::Creator {}),
            label: "Test-Group".to_string(),
        }
    }

    fn mint_into_collection(app: &mut App, owner: String, token_id: String) {
        let mint_msg = Cw721ExecuteMsg::Mint::<Extension, Extension>(MintMsg::<Extension> {
            token_id,
            owner,
            token_uri: None,
            extension: None,
        });

        app.execute_contract(
            Addr::unchecked(MINTER),
            Addr::unchecked(COLLECTION_CONTRACT),
            &mint_msg,
            &[],
        )
        .unwrap();
    }

    fn join_group(app: &mut App, sender: String, token_id: String) {
        let msg = to_binary("This is unused").unwrap();

        let send_nft_msg = Cw721ExecuteMsg::SendNft::<Extension, Extension> {
            contract: SG_NFT_GROUP_CONTRACT.to_string(),
            token_id,
            msg,
        };
        app.execute_contract(
            Addr::unchecked(sender),
            Addr::unchecked(COLLECTION_CONTRACT),
            &send_nft_msg,
            &[],
        )
        .unwrap();
    }

    fn mint_and_join_nft_group(app: &mut App, members: Vec<Member>) {
        for member in members {
            for i in 0..member.weight {
                let token_id = format!("{}/{}", member.clone().addr, i);
                mint_into_collection(app, member.clone().addr, token_id.clone());
                join_group(app, member.clone().addr, token_id);
            }
        }
    }

    #[test]
    fn test_members_assigned_weights() {
        let mut app = mock_app(&[]);

        let init_group = sg_nft_group_init_info(&mut app);
        let init_msg: InstantiateMsg = from_binary(&init_group.msg).unwrap();
        let group_addr = app
            .instantiate_contract(
                init_group.code_id,
                Addr::unchecked(OWNER),
                &init_msg,
                &[],
                init_group.label,
                None,
            )
            .unwrap();

        let version = query_contract_info(&app, group_addr.clone()).unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            version,
        );

        let response: MemberListResponse = app
            .wrap()
            .query_wasm_smart(
                &group_addr,
                &QueryMsg::ListMembers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(response.members, vec![]);

        // All valid
        mint_and_join_nft_group(&mut app, members());

        let response: MemberListResponse = app
            .wrap()
            .query_wasm_smart(
                &group_addr,
                &QueryMsg::ListMembers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(response.members.len(), 3);
    }

    fn setup(app: &mut App) -> Addr {
        let init_group = sg_nft_group_init_info(app);
        let init_msg: InstantiateMsg = from_binary(&init_group.msg).unwrap();
        let group_addr = app
            .instantiate_contract(
                init_group.code_id,
                Addr::unchecked(OWNER),
                &init_msg,
                &[],
                init_group.label,
                None,
            )
            .unwrap();

        mint_and_join_nft_group(app, members());
        group_addr
    }

    #[test]
    fn only_owner_can_remove() {
        let mut app = mock_app(&[]);
        let group_addr = setup(&mut app);

        let msg = QueryMsg::Member {
            addr: OWNER.to_string(),
            at_height: None,
        };
        let response: MemberResponse = app.wrap().query_wasm_smart(&group_addr, &msg).unwrap();
        assert_eq!(response.weight.unwrap(), 1);

        // trying to remove non-existant token ID
        app.execute_contract(
            Addr::unchecked("anyone"),
            group_addr.clone(),
            &ExecuteMsg::Remove {
                token_id: "XXX".to_string(),
            },
            &[],
        )
        .unwrap_err();

        // non owner trying to remove valid token ID
        app.execute_contract(
            Addr::unchecked("anyone"),
            group_addr.clone(),
            &ExecuteMsg::Remove {
                token_id: "XXX".to_string(),
            },
            &[],
        )
        .unwrap_err();

        let token_id = format!("{}/{}", OWNER, 0);
        app.execute_contract(
            Addr::unchecked(OWNER),
            Addr::unchecked(MEMBERSHIP_NFT_CONTRACT),
            &Cw721ExecuteMsg::<Empty, Empty>::Approve {
                spender: group_addr.to_string(),
                token_id: token_id.clone(),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(OWNER),
            group_addr.clone(),
            &ExecuteMsg::Remove { token_id },
            &[],
        )
        .unwrap();

        let response: MemberResponse = app.wrap().query_wasm_smart(&group_addr, &msg).unwrap();
        assert_eq!(response.weight, Some(0));
    }

    #[test]
    fn test_withdrawal() {
        let btc = coin(4, "BTC");
        let mut app = mock_app(&[btc.clone()]);

        let group_addr = setup(&mut app);

        app.execute(
            Addr::unchecked(OWNER),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: group_addr.clone().into(),
                amount: vec![btc.clone()],
            }),
        )
        .unwrap();

        let contract_bal = app.wrap().query_balance(&group_addr, "BTC").unwrap();
        assert_eq!(contract_bal, btc);

        app.execute_contract(
            Addr::unchecked("anyone"),
            group_addr.clone(),
            &ExecuteMsg::Withdraw {
                denom: "BTC".to_string(),
            },
            &[],
        )
        .unwrap();

        let contract_bal = app.wrap().query_balance(&group_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(0, "BTC"));

        for member in members() {
            let response: MemberResponse = app
                .wrap()
                .query_wasm_smart(
                    &group_addr,
                    &QueryMsg::Member {
                        addr: member.clone().addr,
                        at_height: None,
                    },
                )
                .unwrap();
            let bal = app.wrap().query_balance(&member.addr, "BTC").unwrap();
            assert_eq!(Uint128::from(response.weight.unwrap()), bal.amount);
        }
    }
}
