use crate::error::ContractError;
use chess_engine::{Color, Game, GameAction, GameOver};
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

impl From<&str> for CwChessAction {
    fn from(make_move: &str) -> CwChessAction {
        CwChessAction::MakeMove(make_move.to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessColor {
    White,
    Black,
}

impl From<&Color> for CwChessColor {
    fn from(status: &Color) -> CwChessColor {
        match status {
            Color::Black => CwChessColor::Black,
            Color::White => CwChessColor::White,
        }
    }
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
    BlackTimeout,
    WhiteTimeout,
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
    // per player block time limit for all moves
    // starts at first move (not game start_height)
    pub block_time_limit: Option<u64>,
    pub game_id: u64,
    // list of moves
    pub moves: Vec<CwChessMove>,
    // player1 is white
    pub player1: Addr,
    // player2 is black
    pub player2: Addr,
    // status is None while game is being played
    pub status: Option<CwChessGameOver>,
    // when game was created
    pub start_height: u64,
}

impl CwChessGame {
    // check if game timed out based on block_time_limit
    pub fn check_timeout(
        &mut self,
        current_block: u64,
    ) -> Result<&Option<CwChessGameOver>, ContractError> {
        // check if game already over
        if self.status.is_some() {
            return Err(ContractError::GameAlreadyOver {});
        }
        self.status = match self.block_time_limit {
            None => None,
            Some(block_time_limit) => {
                let block_times = self.get_block_times(current_block);
                if block_times.0 > block_time_limit {
                    Some(CwChessGameOver::WhiteTimeout {})
                } else if block_times.1 > block_time_limit {
                    Some(CwChessGameOver::BlackTimeout {})
                } else {
                    None
                }
            }
        };
        Ok(&self.status)
    }

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
        // check if game already over
        if self.status.is_some() {
            return Err(ContractError::GameAlreadyOver {});
        }
        // check if game timed out
        if self.check_timeout(chess_move.block)?.is_some() {
            // check_timeout updates and returns status
            return Ok(&self.status);
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

    pub fn turn_color(&self) -> Option<CwChessColor> {
        match self.status {
            None => match self.moves.len() % 2 {
                0 => Some(CwChessColor::White),
                1 => Some(CwChessColor::Black),
                // rust can't tell this is impossible
                _ => None,
            },
            _ => None,
        }
    }

    // get number of blocks used by each player
    fn get_block_times(&self, current_block: u64) -> (u64, u64) {
        // block times for (white, black)
        let mut block_times: (u64, u64) = (0, 0);
        // block time starts at first move
        if self.moves.is_empty() {
            return block_times;
        }
        let mut blocks: Vec<u64> = self.moves.iter().map(|m| -> u64 { m.block }).collect();
        // if game not over, add current block to end
        if self.status.is_none() {
            blocks.push(current_block);
        }
        for i in 1..blocks.len() {
            let move_time = blocks[i] - blocks[i - 1];
            if i % 2 == 0 {
                block_times.0 += move_time;
            } else {
                block_times.1 += move_time;
            }
        }
        block_times
    }
}
