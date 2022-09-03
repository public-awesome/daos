#[cfg(test)]
mod tests {
    use std::f32::MIN;

    use crate::{
        contract::{CONTRACT_NAME, CONTRACT_VERSION},
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg, VaultResponse},
        ContractError,
    };
    use cosmwasm_std::{
        coin, coins, to_binary, Addr, BankMsg, BlockInfo, Coin, CosmosMsg, Decimal, Empty,
        Timestamp,
    };
    use cw2::{query_contract_info, ContractVersion};
    use cw3::{
        ProposalListResponse, ProposalResponse, Status, Vote, VoteInfo, VoteListResponse,
        VoteResponse, VoterDetail, VoterListResponse, VoterResponse,
    };
    use cw3_flex_multisig::error::ContractError as Cw3FlexMultisigError;
    use cw3_flex_multisig::state::Executor as Cw3Executor;
    use cw4::{Cw4ExecuteMsg, Member, MemberChangedHookMsg, MemberDiff};
    use cw4_group::helpers::Cw4GroupContract;
    use cw721::{Cw721QueryMsg, NftInfoResponse};
    use cw721_base::{
        msg::{ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg},
        Extension, MintMsg,
    };
    use cw_multi_test::{next_block, App, AppBuilder, Contract, ContractWrapper, Executor};
    use cw_utils::{Duration, Expiration, Threshold, ThresholdResponse};

    // const USER: &str = "USER";
    // const ADMIN: &str = "ADMIN";
    // const NATIVE_DENOM: &str = "denom";

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

    #[track_caller]
    fn instantiate_dao(
        app: &mut App,
        group: Addr,
        threshold: Threshold,
        max_voting_period: Duration,
        executor: Option<Cw3Executor>,
    ) -> Addr {
        let dao_id = app.store_code(contract_nft_dao());
        let vault_code_id = app.store_code(contract_cw721());
        let msg = InstantiateMsg {
            group_addr: group.to_string(),
            threshold,
            max_voting_period,
            executor,
            vault_code_id,
        };
        app.instantiate_contract(dao_id, Addr::unchecked(OWNER), &msg, &[], "dao", None)
            .unwrap()
    }

    // this will set up both contracts, instantiating the group with
    // all voters defined above, and the multisig pointing to it and given threshold criteria.
    // Returns (multisig address, group address).
    #[track_caller]
    fn setup_test_case_fixed(
        app: &mut App,
        weight_needed: u64,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
        multisig_as_group_admin: bool,
    ) -> (Addr, Addr) {
        setup_test_case(
            app,
            Threshold::AbsoluteCount {
                weight: weight_needed,
            },
            max_voting_period,
            init_funds,
            multisig_as_group_admin,
            None,
        )
    }

    #[track_caller]
    fn setup_test_case(
        app: &mut App,
        threshold: Threshold,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
        multisig_as_group_admin: bool,
        executor: Option<Cw3Executor>,
    ) -> (Addr, Addr) {
        // 1. Instantiate group contract with members (and OWNER as admin)
        let members = vec![
            member(OWNER, 0),
            member(VOTER1, 1),
            member(VOTER2, 2),
            member(VOTER3, 3),
            member(VOTER4, 12), // so that he alone can pass a 50 / 52% threshold proposal
            member(VOTER5, 5),
        ];
        let group_addr = instantiate_group(app, members);
        app.update_block(next_block);

        // 2. Set up Multisig backed by this group
        let dao_addr = instantiate_dao(
            app,
            group_addr.clone(),
            threshold,
            max_voting_period,
            executor,
        );
        app.update_block(next_block);

        // 3. (Optional) Set the multisig as the group owner
        if multisig_as_group_admin {
            let update_admin = Cw4ExecuteMsg::UpdateAdmin {
                admin: Some(dao_addr.to_string()),
            };
            app.execute_contract(
                Addr::unchecked(OWNER),
                group_addr.clone(),
                &update_admin,
                &[],
            )
            .unwrap();
            app.update_block(next_block);
        }

        // Bonus: set some funds on the multisig contract for future proposals
        if !init_funds.is_empty() {
            app.send_tokens(Addr::unchecked(OWNER), dao_addr.clone(), &init_funds)
                .unwrap();
        }
        (dao_addr, group_addr)
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

    #[test]
    fn test_propose_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let required_weight = 4;
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) =
            setup_test_case_fixed(&mut app, required_weight, voting_period, init_funds, false);

        let proposal = pay_somebody_proposal();
        // Only voters can propose
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &proposal, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        // Wrong expiration option fails
        let msgs = match proposal.clone() {
            ExecuteMsg::Propose { msgs, .. } => msgs,
            _ => panic!("Wrong variant"),
        };
        let proposal_wrong_exp = ExecuteMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs,
            latest: Some(Expiration::AtHeight(123456)),
        };
        let err = app
            .execute_contract(
                Addr::unchecked(OWNER),
                dao_addr.clone(),
                &proposal_wrong_exp,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongExpiration {}),
            err.downcast().unwrap()
        );

        // Proposal from voter works
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "propose"),
                ("sender", VOTER3),
                ("proposal_id", "1"),
                ("status", "Open"),
            ],
        );

        // Proposal from voter with enough vote power directly passes
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr, &proposal, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "propose"),
                ("sender", VOTER4),
                ("proposal_id", "2"),
                ("status", "Passed"),
            ],
        );
    }

    fn expire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => block.time = block.time.plus_seconds(duration + 1),
                Duration::Height(duration) => block.height += duration + 1,
            };
        }
    }

    #[test]
    fn test_proposal_queries() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(80),
            quorum: Decimal::percent(20),
        };
        let (dao_addr, _) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, false, None);

        // create proposal with 1 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id1: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // another proposal immediately passes
        app.update_block(next_block);
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // expire them both
        app.update_block(expire(voting_period));

        // add one more open proposal, 2 votes
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id3: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let proposed_at = app.block_info();

        // next block, let's query them all... make sure status is properly updated (1 should be rejected in query)
        app.update_block(next_block);
        let list_query = QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        };
        let res: ProposalListResponse =
            app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
        assert_eq!(3, res.proposals.len());

        // check the id and status are properly set
        let info: Vec<_> = res.proposals.iter().map(|p| (p.id, p.status)).collect();
        let expected_info = vec![
            (proposal_id1, Status::Rejected),
            (proposal_id2, Status::Passed),
            (proposal_id3, Status::Open),
        ];
        assert_eq!(expected_info, info);

        // ensure the common features are set
        let (expected_msgs, expected_title, expected_description) = proposal_info();
        for prop in res.proposals {
            assert_eq!(prop.title, expected_title);
            assert_eq!(prop.description, expected_description);
            assert_eq!(prop.msgs, expected_msgs);
        }

        // reverse query can get just proposal_id3
        let list_query = QueryMsg::ReverseProposals {
            start_before: None,
            limit: Some(1),
        };
        let res: ProposalListResponse =
            app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
        assert_eq!(1, res.proposals.len());

        let (msgs, title, description) = proposal_info();
        let expected = ProposalResponse {
            id: proposal_id3,
            title,
            description,
            msgs,
            expires: voting_period.after(&proposed_at),
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                total_weight: 23,
                threshold: Decimal::percent(80),
                quorum: Decimal::percent(20),
            },
        };
        assert_eq!(&expected, &res.proposals[0]);
    }

    fn get_tally(app: &App, flex_addr: &str, proposal_id: u64) -> u64 {
        // Get all the voters on the proposal
        let voters = QueryMsg::ListVotes {
            proposal_id,
            start_after: None,
            limit: None,
        };
        let votes: VoteListResponse = app.wrap().query_wasm_smart(flex_addr, &voters).unwrap();
        // Sum the weights of the Yes votes to get the tally
        votes
            .votes
            .iter()
            .filter(|&v| v.vote == Vote::Yes)
            .map(|v| v.weight)
            .sum()
    }

    fn unexpire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => {
                    block.time =
                        Timestamp::from_nanos(block.time.nanos() - (duration * 1_000_000_000));
                }
                Duration::Height(duration) => block.height -= duration,
            };
        }
    }

    #[test]
    fn test_vote_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, false, None);

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Owner with 0 voting power cannot vote
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        // Only voters can vote
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        // But voter1 can
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER1),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Open"),
            ],
        );

        // VOTER1 cannot vote again
        let err = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::AlreadyVoted {}),
            err.downcast().unwrap()
        );

        // No/Veto votes have no effect on the tally
        // Compute the current tally
        let tally = get_tally(&app, dao_addr.as_ref(), proposal_id);
        assert_eq!(tally, 1);

        // Cast a No vote
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &no_vote, &[])
            .unwrap();

        // Cast a Veto vote
        let veto_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Veto,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &veto_vote, &[])
            .unwrap();

        // Tally unchanged
        assert_eq!(tally, get_tally(&app, dao_addr.as_ref(), proposal_id));

        let err = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::AlreadyVoted {}),
            err.downcast().unwrap()
        );

        // Expired proposals cannot be voted
        app.update_block(expire(voting_period));
        let err = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Expired {}),
            err.downcast().unwrap()
        );
        app.update_block(unexpire(voting_period));

        // Powerful voter supports it, so it passes
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // non-Open proposals cannot be voted
        let err = app
            .execute_contract(Addr::unchecked(VOTER5), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::NotOpen {}),
            err.downcast().unwrap()
        );

        // query individual votes
        // initial (with 0 weight)
        let voter = OWNER.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                proposal_id,
                voter: OWNER.into(),
                vote: Vote::Yes,
                weight: 0
            }
        );

        // nay sayer
        let voter = VOTER2.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                proposal_id,
                voter: VOTER2.into(),
                vote: Vote::No,
                weight: 2
            }
        );

        // non-voter
        let voter = VOTER5.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert!(vote.vote.is_none());

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Cast a No vote
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &no_vote, &[])
            .unwrap();

        // Powerful voter opposes it, so it rejects
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr, &no_vote, &[])
            .unwrap();

        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Rejected"),
            ],
        );
    }

    #[test]
    fn test_execute_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, true, None);

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&dao_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongExecuteStatus {}),
            err.downcast().unwrap()
        );

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER4),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // In passing: Try to close Passed fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongCloseStatus {}),
            err.downcast().unwrap()
        );

        // Execute works. Anybody can execute Passed proposals
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &execution, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // verify money was transfered
        let some_bal = app.wrap().query_balance(SOMEBODY, "BTC").unwrap();
        assert_eq!(some_bal, coin(1, "BTC"));
        let contract_bal = app.wrap().query_balance(&dao_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(9, "BTC"));

        // In passing: Try to close Executed fails
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongCloseStatus {}),
            err.downcast().unwrap()
        );

        // Trying to execute something that was already executed fails
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongExecuteStatus {}),
            err.downcast().unwrap()
        );
    }

    #[test]
    fn execute_with_executor_member() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            Some(Cw3Executor::Member), // set executor as Member of voting group
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &vote, &[])
            .unwrap();

        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked("anyone")), // anyone is not allowed to execute
                dao_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        app.execute_contract(
            Addr::unchecked(Addr::unchecked(VOTER2)), // member of voting group is allowed to execute
            dao_addr,
            &execution,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn execute_with_executor_only() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            init_funds,
            true,
            Some(Cw3Executor::Only(Addr::unchecked(VOTER3))), // only VOTER3 can execute proposal
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &vote, &[])
            .unwrap();

        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked("anyone")), // anyone is not allowed to execute
                dao_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        let err = app
            .execute_contract(
                Addr::unchecked(Addr::unchecked(VOTER1)), // VOTER1 is not allowed to execute
                dao_addr.clone(),
                &execution,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        app.execute_contract(
            Addr::unchecked(Addr::unchecked(VOTER3)), // VOTER3 is allowed to execute
            dao_addr,
            &execution,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn proposal_pass_on_expiration() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = 2000000;
        let (dao_addr, _) = setup_test_case(
            &mut app,
            threshold,
            Duration::Time(voting_period),
            init_funds,
            true,
            None,
        );

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&dao_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Vote it, so it passes after voting period is over
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER3),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Open"),
            ],
        );

        // Wait until the voting period is over.
        app.update_block(|block| {
            block.time = block.time.plus_seconds(voting_period);
            block.height += std::cmp::max(1, voting_period / 5);
        });

        // Proposal should now be passed.
        let prop: ProposalResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Proposal { proposal_id })
            .unwrap();
        assert_eq!(prop.status, Status::Passed);

        // Execution should now be possible.
        let res = app
            .execute_contract(
                Addr::unchecked(SOMEBODY),
                dao_addr,
                &ExecuteMsg::Execute { proposal_id },
                &[],
            )
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );
    }

    #[test]
    fn test_close_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Height(2000000);
        let (dao_addr, _) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, true, None);

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Non-expired proposals cannot be closed
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::NotExpired {}),
            err.downcast().unwrap()
        );

        // Expired proposals can be closed
        app.update_block(expire(voting_period));
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "close"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // Trying to close it again fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::WrongCloseStatus {}),
            err.downcast().unwrap()
        );
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn execute_group_changes_from_external() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(20000);
        let (dao_addr, group_addr) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, false, None);

        // VOTER1 starts a proposal to send some tokens (1/4 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };

        // 1/4 votes
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // check current threshold (global)
        let threshold: ThresholdResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Threshold {})
            .unwrap();
        let expected_thresh = ThresholdResponse::ThresholdQuorum {
            total_weight: 23,
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        assert_eq!(expected_thresh, threshold);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group
        // updates VOTER2 power to 21 -> with snapshot, vote doesn't pass proposal
        // adds NEWBIE with 2 power -> with snapshot, invalid vote
        // removes VOTER3 -> with snapshot, can vote on proposal
        let newbie: &str = "newbie";
        let update_msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 21), member(newbie, 2)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // check membership queries properly updated
        let query_voter = QueryMsg::Voter {
            address: VOTER3.into(),
        };
        let power: VoterResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &query_voter)
            .unwrap();
        assert_eq!(power.weight, None);

        // proposal still open
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // make a second proposal
        let proposal2 = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal2, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // VOTER2 can pass this alone with the updated vote (newer height ignores snapshot)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id2), Status::Passed);

        // VOTER2 can only vote on first proposal with weight of 2 (not enough to pass)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, proposal_id), Status::Open);

        // newbie cannot vote
        let err = app
            .execute_contract(Addr::unchecked(newbie), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        // previously removed VOTER3 can still vote, passing the proposal
        app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
            .unwrap();

        // check current threshold (global) is updated
        let threshold: ThresholdResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Threshold {})
            .unwrap();
        let expected_thresh = ThresholdResponse::ThresholdQuorum {
            total_weight: 41,
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        assert_eq!(expected_thresh, threshold);

        // TODO: check proposal threshold not changed
    }

    // uses the power from the beginning of the voting period
    // similar to above - simpler case, but shows that one proposals can
    // trigger the action
    #[test]
    fn execute_group_changes_from_proposal() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let required_weight = 4;
        let voting_period = Duration::Time(20000);
        let (dao_addr, group_addr) =
            setup_test_case_fixed(&mut app, required_weight, voting_period, init_funds, true);

        // Start a proposal to remove VOTER3 from the set
        let update_msg = Cw4GroupContract::new(group_addr)
            .update_members(vec![VOTER3.into()], vec![])
            .unwrap();
        let update_proposal = ExecuteMsg::Propose {
            title: "Kick out VOTER3".to_string(),
            description: "He's trying to steal our money".to_string(),
            msgs: vec![update_msg],
            latest: None,
        };
        let res = app
            .execute_contract(
                Addr::unchecked(VOTER1),
                dao_addr.clone(),
                &update_proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let update_proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER1 starts a proposal to send some tokens
        let cash_proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(
                Addr::unchecked(VOTER1),
                dao_addr.clone(),
                &cash_proposal,
                &[],
            )
            .unwrap();
        // Get the proposal id from the logs
        let cash_proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        assert_ne!(cash_proposal_id, update_proposal_id);

        // query proposal state
        let prop_status = |app: &App, proposal_id: u64| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);
        assert_eq!(prop_status(&app, update_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // Pass and execute first proposal
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: update_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        let execution = ExecuteMsg::Execute {
            proposal_id: update_proposal_id,
        };
        app.execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &execution, &[])
            .unwrap();

        // ensure that the update_proposal is executed, but the other unchanged
        assert_eq!(prop_status(&app, update_proposal_id), Status::Executed);
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Open);

        // next block...
        app.update_block(|b| b.height += 1);

        // VOTER3 can still pass the cash proposal
        // voting on it fails
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: cash_proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app, cash_proposal_id), Status::Passed);

        // but cannot open a new one
        let cash_proposal = pay_somebody_proposal();
        let err = app
            .execute_contract(
                Addr::unchecked(VOTER3),
                dao_addr.clone(),
                &cash_proposal,
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );

        // extra: ensure no one else can call the hook
        let hook_hack = ExecuteMsg::MemberChangedHook(MemberChangedHookMsg {
            diffs: vec![MemberDiff::new(VOTER1, Some(1), None)],
        });
        let err = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &hook_hack, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::Cw3FlexMultisig(Cw3FlexMultisigError::Unauthorized {}),
            err.downcast().unwrap()
        );
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn percentage_handles_group_changes() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 51% required, which is 12 of the initial 24
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(20000);
        let (dao_addr, group_addr) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, false, None);

        // VOTER3 starts a proposal to send some tokens (3/12 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };

        // 3/12 votes
        assert_eq!(prop_status(&app), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group (3 -> 0, 2 -> 9, 0 -> 29) - total = 56, require 29 to pass
        let newbie: &str = "newbie";
        let update_msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 9), member(newbie, 29)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // VOTER2 votes according to original weights: 3 + 2 = 5 / 12 => Open
        // with updated weights, it would be 3 + 9 = 12 / 12 => Passed
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app), Status::Open);

        // new proposal can be passed single-handedly by newbie
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(newbie), dao_addr.clone(), &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // check proposal2 status
        let query_prop = QueryMsg::Proposal {
            proposal_id: proposal_id2,
        };
        let prop: ProposalResponse = app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
        assert_eq!(Status::Passed, prop.status);
    }

    // uses the power from the beginning of the voting period
    #[test]
    fn quorum_handles_group_changes() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 33% required for quora, which is 8 of the initial 24
        // 50% yes required to pass early (12 of the initial 24)
        let voting_period = Duration::Time(20000);
        let (dao_addr, group_addr) = setup_test_case(
            &mut app,
            Threshold::ThresholdQuorum {
                threshold: Decimal::percent(51),
                quorum: Decimal::percent(33),
            },
            voting_period,
            init_funds,
            false,
            None,
        );

        // VOTER3 starts a proposal to send some tokens (3 votes)
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };

        // 3/12 votes - not expired
        assert_eq!(prop_status(&app), Status::Open);

        // a few blocks later...
        app.update_block(|block| block.height += 2);

        // admin changes the group (3 -> 0, 2 -> 9, 0 -> 28) - total = 55, require 28 to pass
        let newbie: &str = "newbie";
        let update_msg = cw4_group::msg::ExecuteMsg::UpdateMembers {
            remove: vec![VOTER3.into()],
            add: vec![member(VOTER2, 9), member(newbie, 29)],
        };
        app.execute_contract(Addr::unchecked(OWNER), group_addr, &update_msg, &[])
            .unwrap();

        // a few blocks later...
        app.update_block(|block| block.height += 3);

        // VOTER2 votes yes, according to original weights: 3 yes, 2 no, 5 total (will fail when expired)
        // with updated weights, it would be 3 yes, 9 yes, 11 total (will pass when expired)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        // not expired yet
        assert_eq!(prop_status(&app), Status::Open);

        // wait until the vote is over, and see it was rejected
        app.update_block(expire(voting_period));
        assert_eq!(prop_status(&app), Status::Rejected);
    }

    #[test]
    fn quorum_enforced_even_if_absolute_threshold_met() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        // 33% required for quora, which is 5 of the initial 15
        // 50% yes required to pass early (8 of the initial 15)
        let voting_period = Duration::Time(20000);
        let (dao_addr, _) = setup_test_case(
            &mut app,
            // note that 60% yes is not enough to pass without 20% no as well
            Threshold::ThresholdQuorum {
                threshold: Decimal::percent(60),
                quorum: Decimal::percent(80),
            },
            voting_period,
            init_funds,
            false,
            None,
        );

        // create proposal
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER5), dao_addr.clone(), &proposal, &[])
            .unwrap();
        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app), Status::Open);
        app.update_block(|block| block.height += 3);

        // reach 60% of yes votes, not enough to pass early (or late)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER4), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        // 9 of 15 is 60% absolute threshold, but less than 12 (80% quorum needed)
        assert_eq!(prop_status(&app), Status::Open);

        // add 3 weight no vote and we hit quorum and this passes
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &no_vote, &[])
            .unwrap();
        assert_eq!(prop_status(&app), Status::Passed);
    }

    // NFT tests ------------------------------------------------------------------

    const TOKEN_ID: &str = "token0001";
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

    #[track_caller]
    fn setup_test_collection(app: &mut App) -> Addr {
        println!("Setup test collection1");
        let collection_addr = instantiate_collection(app);
        app.update_block(next_block);

        // mint an NFT
        let token_id = TOKEN_ID.to_string();
        let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

        let mint_msg = Cw721ExecuteMsg::Mint::<Extension, Extension>(MintMsg::<Extension> {
            token_id,
            owner: OWNER.into(),
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

    #[test]
    fn send_nft_to_dao_works() {
        let init_funds = coins(10, "BTC");
        let mut app = mock_app(&init_funds);

        let collection_addr = setup_test_collection(&mut app);

        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(1),
        };
        let voting_period = Duration::Time(2000000);
        let (dao_addr, _) =
            setup_test_case(&mut app, threshold, voting_period, init_funds, true, None);

        // ensure we have cash to cover the proposal
        let contract_bal = app.wrap().query_balance(&dao_addr, "BTC").unwrap();
        assert_eq!(contract_bal, coin(10, "BTC"));

        // send NFT from collection to DAO
        let msg = to_binary("You now have the melting power").unwrap();
        let send_msg: Cw721ExecuteMsg<Extension, Extension> = Cw721ExecuteMsg::SendNft {
            contract: dao_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            msg,
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            collection_addr.clone(),
            &send_msg,
            &[],
        )
        .unwrap();

        let res: VaultResponse = app
            .wrap()
            .query_wasm_smart(dao_addr, &QueryMsg::Vault {})
            .unwrap();
        let vault_addr = res.addr;
        assert_eq!("contract3".to_string(), vault_addr);

        let res: NftInfoResponse<Extension> = app
            .wrap()
            .query_wasm_smart(
                vault_addr,
                &Cw721QueryMsg::NftInfo {
                    token_id: TOKEN_ID.to_string(),
                },
            )
            .unwrap();
        // the token_uri is the original collection address
        assert_eq!(collection_addr, res.token_uri.unwrap());
    }

    // fn proposal_nft_transfer_info() -> (Vec<CosmosMsg<Empty>>, String, String) {
    //     // TODO: To transfer and NFT via a proposal, we need two messages.
    //     // 1. A send() message to the collection contract with the NFT
    //     // 2. A transfer() includeded in the send to transfer ownership to the recipient

    //     let transfer_msg = Cw721ExecuteMsg::TransferNft {
    //         recipient: SOMEBODY.into(),
    //         token_id: TOKEN_ID.into(),
    //     };

    //     // TODO: how do you convert a Cw721ExecuteMsg to a CosmosMsg?

    //     let msgs = vec![transfer_msg.into()];
    //     let title = "Transfer NFT".to_string();
    //     let description = "Should we transfer this?".to_string();
    //     (msgs, title, description)
    // }

    // fn transfer_nft_proposal() -> ExecuteMsg {
    //     let (msgs, title, description) = proposal_info();
    //     ExecuteMsg::Propose {
    //         title,
    //         description,
    //         msgs,
    //         latest: None,
    //     }
    // }

    fn proposal_nft_transfer_works() {}
}
