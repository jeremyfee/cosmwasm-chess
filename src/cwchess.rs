use crate::error::ContractError;
use chess::{ChessMove, Color, Game, GameResult};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessAction {
    AcceptDraw,
    DeclareDraw,
    MakeMove(String),
    OfferDraw,
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
pub enum CwChessResult {
    // chess results
    WhiteCheckmates,
    WhiteResigns,
    BlackCheckmates,
    BlackResigns,
    Stalemate,
    DrawAccepted,
    DrawDeclared,
    // custom results
}

impl CwChessResult {
    fn from_result(result: Option<GameResult>) -> Option<CwChessResult> {
        match result {
            None => None,
            Some(GameResult::WhiteCheckmates) => Some(CwChessResult::WhiteCheckmates),
            Some(GameResult::WhiteResigns) => Some(CwChessResult::WhiteResigns),
            Some(GameResult::BlackCheckmates) => Some(CwChessResult::BlackCheckmates),
            Some(GameResult::BlackResigns) => Some(CwChessResult::BlackResigns),
            Some(GameResult::Stalemate) => Some(CwChessResult::Stalemate),
            Some(GameResult::DrawAccepted) => Some(CwChessResult::DrawAccepted),
            Some(GameResult::DrawDeclared) => Some(CwChessResult::DrawDeclared),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessMove {
    pub action: CwChessAction,
    pub block: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessGame {
    pub block_time_limit: Option<u64>,
    pub game_id: u64,
    pub moves: Vec<CwChessMove>,
    pub player1: Addr,
    pub player2: Addr,
    pub result: Option<CwChessResult>,
    pub start_height: u64,
}

impl CwChessGame {
    fn do_move(mut game: Game, chess_move: &CwChessMove) -> Result<Game, ContractError> {
        let ok = match &chess_move.action {
            CwChessAction::MakeMove(movestr) => {
                match ChessMove::from_san(&game.current_position(), movestr) {
                    Ok(chess_move) => game.make_move(chess_move),
                    _ => false,
                }
            }
            CwChessAction::OfferDraw => game.offer_draw(game.current_position().side_to_move()),
            CwChessAction::AcceptDraw => game.accept_draw(),
            CwChessAction::DeclareDraw => game.declare_draw(),
            CwChessAction::Resign => game.resign(game.current_position().side_to_move()),
        };
        if ok {
            Ok(game)
        } else {
            Err(ContractError::InvalidMove {})
        }
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
        let mut game: Game = Game::new();
        for chess_move in self.moves.clone() {
            game = CwChessGame::do_move(game, &chess_move)?;
        }
        Ok(game)
    }

    pub fn make_move(
        &mut self,
        player: &Addr,
        chess_move: CwChessMove,
    ) -> Result<Option<CwChessResult>, ContractError> {
        if self.result.is_some() {
            return Err(ContractError::GameAlreadyFinished {});
        }
        let mut game = self.load_game()?;
        let player_to_move = match game.side_to_move() {
            Color::White => &self.player1,
            Color::Black => &self.player2,
        };
        if player_to_move != player {
            Err(ContractError::NotYourTurn {})
        } else {
            game = CwChessGame::do_move(game, &chess_move)?;
            // save move
            self.moves.push(chess_move);
            // update result in case game ended
            self.result = CwChessResult::from_result(game.result());
            Ok(self.result.clone())
        }
    }
}
