#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        contract::{execute, instantiate, query, reply, CONTRACT_NAME, CONTRACT_VERSION},
        msg::{InstantiateMsg, QueryMsg},
    };
    use cosmwasm_std::{from_binary, to_binary, Addr, Coin, Empty};
    use cw2::{query_contract_info, ContractVersion};
    use cw4::{Member, MemberListResponse};
    use cw721_base::{
        msg::{ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg},
        Extension, MintMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use sg_daos::{Admin, ContractInstantiateMsg};

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const VOTER4: &str = "voter0004";
    const VOTER5: &str = "voter0005";

    const COLLECTION_CONTRACT: &str = "contract0";
    const SG_NFT_GROUP_CONTRACT: &str = "contract1";

    fn member<T: Into<String>>(addr: T, weight: u64) -> Member {
        Member {
            addr: addr.into(),
            weight,
        }
    }

    fn members() -> Vec<Member> {
        vec![
            member(OWNER, 1),
            member(VOTER1, 1),
            member(VOTER2, 2),
            member(VOTER3, 3),
            member(VOTER4, 12),
            member(VOTER5, 5),
        ]
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
        assert_eq!(response.members.len(), 6);
    }
}
