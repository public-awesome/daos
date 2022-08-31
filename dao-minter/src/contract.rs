use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg};
use crate::state::{increment_token_index, Config, COLLECTION_ADDRESS, CONFIG, STATUS};

use base_factory::msg::{BaseMinterCreateMsg, Extension, ParamsResponse};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use cw3_flex_multisig::msg::InstantiateMsg;
use cw_utils::{must_pay, parse_reply_instantiate_data, Duration, Threshold};

use sg1::checked_fair_burn;
use sg2::query::Sg2QueryMsg;
use sg4::{QueryMsg, Status, StatusResponse, SudoMsg};
use sg721::{ExecuteMsg as Sg721ExecuteMsg, InstantiateMsg as Sg721InstantiateMsg, MintMsg};
use sg_std::math::U64Ext;
use sg_std::{Response, SubMsg, NATIVE_DENOM};

const CONTRACT_NAME: &str = "crates.io:sg-dao-minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const COLLECTION_INIT_REPLY_ID: u64 = 1;
const DAO_INIT_REPLY_ID: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: BaseMinterCreateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let factory = info.sender.clone();

    // set default status so it can be queried without failing
    STATUS.save(deps.storage, &Status::default())?;

    // Make sure the sender is the factory contract
    // This will fail if the sender cannot parse a response from the factory contract
    let res: ParamsResponse = deps
        .querier
        .query_wasm_smart(factory.clone(), &Sg2QueryMsg::Params {})?;

    let config = Config {
        factory: factory.clone(),
        collection_code_id: msg.collection_params.code_id,
        // assume the mint price is the minimum mint price
        // 100% is fair burned
        mint_price: res.params.min_mint_price,
        extension: Empty {},
    };

    CONFIG.save(deps.storage, &config)?;

    let wasm_msg = WasmMsg::Instantiate {
        code_id: msg.collection_params.code_id,
        msg: to_binary(&Sg721InstantiateMsg {
            name: msg.collection_params.name.clone(),
            symbol: msg.collection_params.symbol,
            minter: env.contract.address.to_string(),
            collection_info: msg.collection_params.info,
        })?,
        funds: info.funds,
        admin: None,
        label: format!(
            "SG721-{}-{}",
            msg.collection_params.code_id,
            msg.collection_params.name.trim()
        ),
    };
    let submsg = SubMsg::reply_on_success(wasm_msg, COLLECTION_INIT_REPLY_ID);

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("sender", factory)
        .add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {
            group_addr,
            threshold,
            max_voting_period,
            dao_code_id,
        } => execute_mint_sender(
            deps,
            info,
            group_addr,
            threshold,
            max_voting_period,
            dao_code_id,
        ),
    }
}

pub fn execute_mint_sender(
    deps: DepsMut,
    info: MessageInfo,
    group_addr: String,
    threshold: Threshold,
    max_voting_period: Duration,
    dao_code_id: u64,
) -> Result<Response, ContractError> {
    let mut res = Response::new();

    let config = CONFIG.load(deps.storage)?;
    let collection = COLLECTION_ADDRESS.load(deps.storage)?;

    let factory: ParamsResponse = deps
        .querier
        .query_wasm_smart(config.factory, &Sg2QueryMsg::Params {})?;
    let factory_params = factory.params;

    let funds_sent = must_pay(&info, NATIVE_DENOM)?;

    // Create network fee msgs
    let mint_fee_percent = factory_params.mint_fee_bps.bps_to_decimal();
    let network_fee = config.mint_price.amount * mint_fee_percent;
    // For the base 1/1 minter, the entire mint price should be Fair Burned
    if network_fee != funds_sent {
        return Err(ContractError::InvalidMintPrice {});
    }
    checked_fair_burn(&info, network_fee.u128(), None, &mut res)?;

    // TODO: verify group address is a valid group
    // TODO: verify sender is the group admin
    // TODO: move dao_code_id, threshold, max_voting_period into config?

    // instantiate cw3-flex-multisig
    let init_msg = InstantiateMsg {
        group_addr: group_addr.clone(),
        threshold,
        max_voting_period,
        executor: None,
    };
    let wasm_msg = WasmMsg::Instantiate {
        admin: Some(group_addr),
        code_id: dao_code_id,
        msg: to_binary(&init_msg)?,
        funds: vec![],
        label: format!("DAO-{}", dao_code_id),
    };
    res = res.add_submessage(SubMsg::reply_on_success(wasm_msg, DAO_INIT_REPLY_ID));

    Ok(res
        .add_attribute("action", "mint")
        .add_attribute("sender", info.sender)
        .add_attribute("network_fee", network_fee.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::UpdateStatus {
            is_verified,
            is_blocked,
            is_explicit,
        } => update_status(deps, is_verified, is_blocked, is_explicit)
            .map_err(|_| ContractError::UpdateStatus {}),
    }
}

/// Only governance can update contract params
pub fn update_status(
    deps: DepsMut,
    is_verified: bool,
    is_blocked: bool,
    is_explicit: bool,
) -> StdResult<Response> {
    let mut status = STATUS.load(deps.storage)?;
    status.is_verified = is_verified;
    status.is_blocked = is_blocked;
    status.is_explicit = is_explicit;

    Ok(Response::new().add_attribute("action", "sudo_update_status"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Status {} => to_binary(&query_status(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let collection_address = COLLECTION_ADDRESS.load(deps.storage)?;

    Ok(ConfigResponse {
        collection_address: collection_address.to_string(),
        config,
    })
}

pub fn query_status(deps: Deps) -> StdResult<StatusResponse> {
    let status = STATUS.load(deps.storage)?;

    Ok(StatusResponse { status })
}

// Reply callback triggered from sg721 contract instantiation in instantiate()
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let reply = parse_reply_instantiate_data(msg.clone());

    match reply {
        Ok(res) => {
            match msg.id {
                COLLECTION_INIT_REPLY_ID => {
                    let collection_addr = res.contract_address;

                    // mark the collection contract as ready to mint
                    let msg = WasmMsg::Execute {
                        contract_addr: collection_addr.clone(),
                        msg: to_binary(&Sg721ExecuteMsg::<Empty>::_Ready {})?,
                        funds: vec![],
                    };

                    COLLECTION_ADDRESS.save(deps.storage, &Addr::unchecked(collection_addr))?;

                    Ok(Response::default()
                        .add_attribute("action", "reply_on_success")
                        .add_message(msg))
                }
                DAO_INIT_REPLY_ID => {
                    let dao_addr = res.contract_address;
                    // TODO: query dao group for nft owner
                    let group_addr = "group_app".to_string();
                    let collection = COLLECTION_ADDRESS.load(deps.storage)?;

                    // Create mint msgs
                    let mint_msg = Sg721ExecuteMsg::Mint(MintMsg::<Extension> {
                        token_id: increment_token_index(deps.storage)?.to_string(),
                        owner: group_addr,
                        token_uri: Some(dao_addr),
                        extension: None,
                    });
                    let msg = WasmMsg::Execute {
                        contract_addr: collection.to_string(),
                        msg: to_binary(&mint_msg)?,
                        funds: vec![],
                    };
                    Ok(Response::default()
                        .add_attribute("action", "reply_on_success")
                        .add_message(msg))
                }
                _ => Err(ContractError::InvalidReplyID {}),
            }
        }
        Err(_) => Err(ContractError::ReplyOnSuccess {}),
    }
}
