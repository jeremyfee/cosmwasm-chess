use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cwchess::{CwChessAction, CwChessColor};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateChallenge {
        opponent: Option<String>,
        play_as: Option<CwChessColor>,
        block_time_limit: Option<u64>,
        // sender is creator
    },
    AcceptChallenge {
        challenge_id: u64,
        // sender is player
    },
    CancelChallenge {
        challenge_id: u64,
        // sender is creator
    },
    Move {
        game_id: u64,
        action: CwChessAction,
        // sender is player
        // block is timestamp
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetChallenge { challenge_id: u64 },
    GetGame { game_id: u64 },
    GetPlayerInfo { player: String },
}
