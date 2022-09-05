#[cfg(test)]
mod tests {
    use crate::msg::{InstantiateMsg, QueryMsg};

    use cosmwasm_std::{Addr, Coin, Empty};
    use cw4::{MemberListResponse, MemberResponse, TotalWeightResponse};
    use cw721_base::{
        msg::{ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg},
        Extension, MintMsg,
    };
    use cw_controllers::AdminResponse;
    use cw_multi_test::{next_block, App, AppBuilder, Contract, ContractWrapper, Executor};

    const INIT_ADMIN: &str = "juan";
    const NFT_OWNER1: &str = "somebody";
    const NFT_OWNER2: &str = "else";
    const NFT_OWNER3: &str = "funny";
    const TOKEN_ID1: &str = "token0001";
    const TOKEN_ID2: &str = "token0002";
    const TOKEN_ID3: &str = "token0003";
    const MINTER: &str = "minter";

    pub fn contract_nft_group() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
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
            Addr::unchecked(INIT_ADMIN),
            &msg,
            &[],
            "collection",
            None,
        )
        .unwrap()
    }

    #[track_caller]
    fn setup_test_collection(app: &mut App) -> Addr {
        let collection_addr = instantiate_collection(app);
        app.update_block(next_block);

        // mint NFTs
        let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

        let mint_msg = Cw721ExecuteMsg::Mint::<Extension, Extension>(MintMsg::<Extension> {
            token_id: TOKEN_ID1.to_string(),
            owner: NFT_OWNER1.into(),
            token_uri: Some(token_uri.clone()),
            extension: None,
        });
        app.execute_contract(
            Addr::unchecked(MINTER),
            collection_addr.clone(),
            &mint_msg,
            &[],
        )
        .unwrap();

        let mint_msg = Cw721ExecuteMsg::Mint::<Extension, Extension>(MintMsg::<Extension> {
            token_id: TOKEN_ID2.to_string(),
            owner: NFT_OWNER2.into(),
            token_uri: Some(token_uri.clone()),
            extension: None,
        });
        app.execute_contract(
            Addr::unchecked(MINTER),
            collection_addr.clone(),
            &mint_msg,
            &[],
        )
        .unwrap();

        let mint_msg = Cw721ExecuteMsg::Mint::<Extension, Extension>(MintMsg::<Extension> {
            token_id: TOKEN_ID3.to_string(),
            owner: NFT_OWNER2.into(),
            token_uri: Some(token_uri),
            extension: None,
        });
        app.execute_contract(
            Addr::unchecked(MINTER),
            collection_addr.clone(),
            &mint_msg,
            &[],
        )
        .unwrap();

        collection_addr
    }

    #[track_caller]
    fn instantiate_group(app: &mut App, admin: Option<String>, collection_addr: String) -> Addr {
        let group_id = app.store_code(contract_nft_group());

        let msg = InstantiateMsg {
            admin,
            collection_addr,
        };
        app.instantiate_contract(
            group_id,
            Addr::unchecked(INIT_ADMIN),
            &msg,
            &[],
            "nft-group",
            None,
        )
        .unwrap()
    }

    fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(INIT_ADMIN), init_funds.to_vec())
                .unwrap();
        })
    }

    #[test]
    fn test_instantiate_works() {
        let mut app = mock_app(&[]);

        let collection_addr = setup_test_collection(&mut app);
        let group = instantiate_group(&mut app, None, collection_addr.to_string());

        let res: AdminResponse = app
            .wrap()
            .query_wasm_smart(group.clone(), &QueryMsg::Admin {})
            .unwrap();
        assert_eq!(None, res.admin);
        // assert_eq!(Some(INIT_ADMIN.into()), res.admin)

        let res: TotalWeightResponse = app
            .wrap()
            .query_wasm_smart(group, &QueryMsg::TotalWeight {})
            .unwrap();
        assert_eq!(3, res.weight);
    }

    #[test]
    fn try_member_queries() {
        let mut app = mock_app(&[]);

        let collection_addr = setup_test_collection(&mut app);
        let group = instantiate_group(&mut app, None, collection_addr.to_string());

        let res: MemberResponse = app
            .wrap()
            .query_wasm_smart(
                group.clone(),
                &QueryMsg::Member {
                    addr: NFT_OWNER1.into(),
                    at_height: None,
                },
            )
            .unwrap();
        assert_eq!(res.weight, Some(1));

        let res: MemberResponse = app
            .wrap()
            .query_wasm_smart(
                group.clone(),
                &QueryMsg::Member {
                    addr: NFT_OWNER2.into(),
                    at_height: None,
                },
            )
            .unwrap();
        assert_eq!(res.weight, Some(2));

        let res: MemberResponse = app
            .wrap()
            .query_wasm_smart(
                group.clone(),
                &QueryMsg::Member {
                    addr: NFT_OWNER3.into(),
                    at_height: None,
                },
            )
            .unwrap();
        assert_eq!(res.weight, None);

        let res: MemberListResponse = app
            .wrap()
            .query_wasm_smart(
                group,
                &QueryMsg::ListMembers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(res.members.len(), 2);
    }
}
