#[cfg(test)]
mod tests {
    use crate::{
        contract::{CONTRACT_NAME, CONTRACT_VERSION},
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
        ContractError,
    };
    use cosmwasm_std::{coins, Addr, BankMsg, Coin, CosmosMsg, Decimal, Empty};
    use cw2::{query_contract_info, ContractVersion};
    use cw3::{VoterDetail, VoterListResponse};
    use cw4::Member;
    use cw721_base::Cw721Contract;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use cw_utils::{Duration, Threshold};

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";

    const SYMBOL: &str = "SGZ";
    const MINTER: &str = "merkle";

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const VOTER4: &str = "voter0004";
    const VOTER5: &str = "voter0005";
    const SOMEBODY: &str = "somebody";

    fn member<T: Into<String>>(addr: T, weight: u64) -> Member {
        Member {
            addr: addr.into(),
            weight,
        }
    }

    pub fn contract_nft_dao() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    pub fn contract_group() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
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

    fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
                .unwrap();
        })
    }

    // uploads code and returns address of group contract
    fn instantiate_group(app: &mut App, members: Vec<Member>) -> Addr {
        let group_id = app.store_code(contract_group());
        let msg = cw4_group::msg::InstantiateMsg {
            admin: Some(OWNER.into()),
            members,
        };
        app.instantiate_contract(group_id, Addr::unchecked(OWNER), &msg, &[], "group", None)
            .unwrap()
    }

    fn proposal_info() -> (Vec<CosmosMsg<Empty>>, String, String) {
        let bank_msg = BankMsg::Send {
            to_address: SOMEBODY.into(),
            amount: coins(1, "BTC"),
        };
        let msgs = vec![bank_msg.into()];
        let title = "Pay somebody".to_string();
        let description = "Do I pay her?".to_string();
        (msgs, title, description)
    }

    fn pay_somebody_proposal() -> ExecuteMsg {
        let (msgs, title, description) = proposal_info();
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest: None,
        }
    }

    #[test]
    fn test_instantiate_works() {
        let mut app = mock_app(&[]);

        // make a simple group
        let group_addr = instantiate_group(&mut app, vec![member(OWNER, 1)]);
        let nft_dao_id = app.store_code(contract_nft_dao());
        let vault_code_id = app.store_code(contract_cw721());

        let max_voting_period = Duration::Time(1234567);

        // Zero required weight fails
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            threshold: Threshold::ThresholdQuorum {
                threshold: Decimal::zero(),
                quorum: Decimal::percent(1),
            },
            max_voting_period,
            executor: None,
            vault_code_id,
        };
        let err = app
            .instantiate_contract(
                nft_dao_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "zero required weight",
                None,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Threshold(cw_utils::ThresholdError::InvalidThreshold {}),
            err.downcast().unwrap()
        );

        // Total weight less than required weight not allowed
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 100 },
            max_voting_period,
            executor: None,
            vault_code_id,
        };
        let err = app
            .instantiate_contract(
                nft_dao_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "high required weight",
                None,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Threshold(cw_utils::ThresholdError::UnreachableWeight {}),
            err.downcast().unwrap()
        );

        // All valid
        let instantiate_msg = InstantiateMsg {
            group_addr: group_addr.to_string(),
            threshold: Threshold::AbsoluteCount { weight: 1 },
            max_voting_period,
            executor: None,
            vault_code_id,
        };
        let dao_addr = app
            .instantiate_contract(
                nft_dao_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "all good",
                None,
            )
            .unwrap();

        // Verify contract version set properly
        let version = query_contract_info(&app, dao_addr.clone()).unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            version,
        );

        // Get voters query
        let voters: VoterListResponse = app
            .wrap()
            .query_wasm_smart(
                &dao_addr,
                &QueryMsg::ListVoters {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(
            voters.voters,
            vec![VoterDetail {
                addr: OWNER.into(),
                weight: 1
            }]
        );
    }
}
