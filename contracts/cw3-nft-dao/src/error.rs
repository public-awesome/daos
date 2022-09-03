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

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid reply ID")]
    InvalidReplyID {},

    #[error("Reply error")]
    ReplyOnSuccess {},
}
