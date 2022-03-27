use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessGameOver};

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
    GetChallenge {
        challenge_id: u64,
    },
    GetChallenges {
        after: Option<u64>,
        player: Option<String>,
    },
    GetGame {
        game_id: u64,
    },
    GetGames {
        after: Option<u64>,
        game_over: Option<bool>,
        player: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GameSummary {
    pub block_time_limit: Option<u64>,
    pub game_id: u64,
    pub player1: String,
    pub player2: String,
    pub status: Option<CwChessGameOver>,
    pub start_height: u64,
    pub turn_color: Option<CwChessColor>,
}

impl From<&CwChessGame> for GameSummary {
    fn from(game: &CwChessGame) -> GameSummary {
        GameSummary {
            block_time_limit: game.block_time_limit,
            game_id: game.game_id,
            player1: game.player1.to_string(),
            player2: game.player2.to_string(),
            status: game.status.clone(),
            start_height: game.start_height,
            turn_color: game.turn_color.clone(),
        }
    }
}
