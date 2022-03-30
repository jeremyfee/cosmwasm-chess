use crate::error::ContractError;
use chess_engine::{Color, Game, GameAction, GameOver};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessAction {
    AcceptDraw,
    #[serde(rename = "move")]
    MakeMove(String),
    OfferDraw(String),
    Resign,
}

impl From<&str> for CwChessAction {
    fn from(make_move: &str) -> CwChessAction {
        CwChessAction::MakeMove(make_move.to_string())
    }
}

impl From<&CwChessAction> for GameAction {
    fn from(action: &CwChessAction) -> GameAction {
        match action {
            CwChessAction::AcceptDraw => GameAction::AcceptDraw,
            CwChessAction::MakeMove(move_str) => GameAction::MakeMove(move_str.to_string()),
            CwChessAction::OfferDraw(move_str) => GameAction::OfferDraw(move_str.to_string()),
            CwChessAction::Resign => GameAction::Resign,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessColor {
    White,
    Black,
}

impl From<&Color> for CwChessColor {
    fn from(color: &Color) -> CwChessColor {
        match color {
            Color::Black => CwChessColor::Black,
            Color::White => CwChessColor::White,
        }
    }
}

impl From<&CwChessColor> for Color {
    fn from(color: &CwChessColor) -> Color {
        match color {
            CwChessColor::Black => Color::Black,
            CwChessColor::White => Color::White,
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

pub type CwChessMove = (u64, CwChessAction);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessGame {
    // per player block limit for all moves
    // starts at first move (not game start_height)
    pub block_limit: Option<u64>,
    // when game was created
    pub block_start: u64,
    // board position in FEN
    // cheaper to load board than executing moves
    pub fen: String,
    // game id
    pub game_id: u64,
    // list of moves
    pub moves: Vec<CwChessMove>,
    // player1 is white
    pub player1: Addr,
    // player2 is black
    pub player2: Addr,
    // status is None while game is being played
    pub status: Option<CwChessGameOver>,
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
        self.status = match self.block_limit {
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
        match Game::from_fen(
            &self.fen,
            self.draw_offered().as_ref().map(Color::from),
            None,
        ) {
            Ok(game) => Ok(game),
            Err(_) => Err(ContractError::InvalidPosition {}),
        }
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
        if self.check_timeout(chess_move.0)?.is_some() {
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
        match game.make_move(&GameAction::from(&chess_move.1)) {
            Err(_) => Err(ContractError::InvalidMove {}),
            Ok(status) => {
                self.moves.push(chess_move);
                self.status = status.as_ref().map(CwChessGameOver::from);
                self.fen = game.to_fen(0, (self.moves.len() / 2) as u8).unwrap();
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

    // check whether draw was offered on previous turn
    // return color that offered draw
    fn draw_offered(&self) -> Option<CwChessColor> {
        match &self.moves.last() {
            Some((_, CwChessAction::OfferDraw(_))) => {
                match self.turn_color() {
                    None => None,
                    // current turn means opposite color offered draw
                    Some(CwChessColor::Black) => Some(CwChessColor::White),
                    Some(CwChessColor::White) => Some(CwChessColor::Black),
                }
            }
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
        let mut blocks: Vec<u64> = self.moves.iter().map(|m| -> u64 { m.0 }).collect();
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
