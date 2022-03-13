use crate::chess_engine_game::{Game, GameAction, GameOver};
use crate::error::ContractError;
use chess_engine::Color;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessAction {
    AcceptDraw,
    MakeMove(String),
    OfferDraw(String),
    Resign,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessColor {
    White,
    Black,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessGameOver {
    // chess_engine_game results
    BlackCheckmates,
    BlackResigns,
    DrawAccepted,
    DrawDeclared,
    Stalemate,
    WhiteCheckmates,
    WhiteResigns,
    // custom results
}

impl From<&GameOver> for CwChessGameOver {
    fn from(status: &GameOver) -> CwChessGameOver {
        match status {
            GameOver::BlackCheckmates => CwChessGameOver::BlackCheckmates,
            GameOver::BlackResigns => CwChessGameOver::BlackResigns,
            GameOver::DrawAccepted => CwChessGameOver::DrawAccepted,
            GameOver::Stalemate => CwChessGameOver::Stalemate,
            GameOver::WhiteCheckmates => CwChessGameOver::WhiteCheckmates,
            GameOver::WhiteResigns => CwChessGameOver::WhiteResigns,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessMove {
    pub action: CwChessAction,
    pub block: u64,
}

impl From<&CwChessMove> for GameAction {
    fn from(chess_move: &CwChessMove) -> GameAction {
        match &chess_move.action {
            CwChessAction::AcceptDraw => GameAction::AcceptDraw,
            CwChessAction::MakeMove(move_str) => GameAction::MakeMove(move_str.to_string()),
            CwChessAction::OfferDraw(move_str) => GameAction::OfferDraw(move_str.to_string()),
            CwChessAction::Resign => GameAction::Resign,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessGame {
    pub block_time_limit: Option<u64>,
    pub game_id: u64,
    pub moves: Vec<CwChessMove>,
    pub player1: Addr,
    pub player2: Addr,
    pub status: Option<CwChessGameOver>,
    pub start_height: u64,
}

impl CwChessGame {
    pub fn get_player_order(
        player1: Addr,
        player2: Addr,
        play_as: Option<CwChessColor>,
        height: u64,
    ) -> (Addr, Addr) {
        match play_as {
            Some(CwChessColor::White) => (player1, player2),
            Some(CwChessColor::Black) => (player2, player1),
            None => {
                if height % 2 == 0 {
                    (player1, player2)
                } else {
                    (player2, player1)
                }
            }
        }
    }

    pub fn load_game(&self) -> Result<Game, ContractError> {
        let mut game: Game = Game::default();
        for chess_move in &self.moves {
            if game.make_move(&GameAction::from(chess_move)).is_err() {
                return Err(ContractError::InvalidMove {});
            }
        }
        Ok(game)
    }

    pub fn make_move(
        &mut self,
        player: &Addr,
        chess_move: CwChessMove,
    ) -> Result<&Option<CwChessGameOver>, ContractError> {
        if self.status.is_some() {
            return Err(ContractError::GameAlreadyFinished {});
        }
        let mut game = self.load_game()?;
        let player_to_move = match game.get_turn_color() {
            Color::White => &self.player1,
            Color::Black => &self.player2,
        };
        if player_to_move != player {
            return Err(ContractError::NotYourTurn {});
        }
        match game.make_move(&GameAction::from(&chess_move)) {
            Err(_) => Err(ContractError::InvalidMove {}),
            Ok(status) => {
                self.moves.push(chess_move);
                self.status = status.as_ref().map(CwChessGameOver::from);
                Ok(&self.status)
            }
        }
    }
}
