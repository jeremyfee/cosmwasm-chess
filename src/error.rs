use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("cannot play self")]
    CannotPlaySelf {},
    #[error("challenge not found")]
    ChallengeNotFound {},
    #[error("game already over")]
    GameAlreadyOver {},
    #[error("game not found")]
    GameNotFound {},
    #[error("game not timed out")]
    GameNotTimedOut {},
    #[error("invalid move")]
    InvalidMove {},
    #[error("invalid position")]
    InvalidPosition {},
    #[error("not your challenge")]
    NotYourChallenge {},
    #[error("not your turn")]
    NotYourTurn {},
}
