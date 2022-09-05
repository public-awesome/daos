use cosmwasm_std::StdError;
use cw3_flex_multisig::ContractError as Cw3FlexMultisigError;
use cw_utils::ThresholdError;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Cw3FlexMultisig(#[from] Cw3FlexMultisigError),

    #[error("{0}")]
    Threshold(#[from] ThresholdError),

    #[error("Invalid reply ID")]
    InvalidReplyID {},

    #[error("Reply error")]
    ReplyOnSuccess {},

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal is not open")]
    NotOpen {},

    #[error("Proposal voting period has expired")]
    Expired {},

    #[error("Proposal must expire before you can close it")]
    NotExpired {},

    #[error("Wrong expiration option")]
    WrongExpiration {},

    #[error("Already voted on this proposal")]
    AlreadyVoted {},

    #[error("Proposal must have passed and not yet been executed")]
    WrongExecuteStatus {},

    #[error("Cannot close completed or passed proposals")]
    WrongCloseStatus {},
}
