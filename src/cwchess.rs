use crate::error::ContractError;
use chess_engine::{Board, Color, GameResult, Move};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CwChessAction {
    AcceptDraw,
    DeclareDraw,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CwChessMove {
    pub action: CwChessAction,
    pub block: u64,
}

// internal struct to simulate (partial) chess::Game interface using chess_engine::Board
pub struct Game {
  pub board: Board,
  pub draw_offered: Option<Color>,
  pub result: Option<CwChessResult>,
}

impl Game {
  pub fn make_move(&mut self, chess_move: &CwChessMove) -> Result<Option<CwChessResult>, ContractError> {
    if self.result.is_some() {
      return Err(ContractError::GameAlreadyFinished{});
    }
    match &chess_move.action {
        CwChessAction::MakeMove(movestr) => self.do_move(movestr.to_string()),
        CwChessAction::OfferDraw(movestr) => {
          let offered_draw = Some(self.side_to_move());
          self.do_move(movestr.to_string())?;
          self.draw_offered = offered_draw;
          Ok(None)
        }
        CwChessAction::AcceptDraw => self.accept_draw(),
        CwChessAction::DeclareDraw => self.declare_draw(),
        CwChessAction::Resign => self.resign(),
    }
  }

  pub fn new() -> Self {
    Game {
      board: Board::default(),
      draw_offered: None,
      result: None
    }
  }


  pub fn side_to_move(&self) -> Color {
    self.board.get_turn_color()
  }

  fn accept_draw(&mut self) -> Result<Option<CwChessResult>, ContractError> {
    if let Some(color) = self.draw_offered {
      if color != self.side_to_move() {
        self.result = Some(CwChessResult::DrawAccepted);
        return Ok(self.result.clone());
      }
    }
    Err(ContractError::InvalidMove{})
  }

  fn declare_draw(&mut self) -> Result<Option<CwChessResult>, ContractError> {
    // TODO implement draw checks
    Err(ContractError::InvalidMove{})
  }

  fn do_move(&mut self, movestr: String) -> Result<Option<CwChessResult>, ContractError> {
    match Move::parse(movestr) {
      Ok(chess_move) => {
        self.result = match self.board.play_move(chess_move) {
          GameResult::Continuing(board) => {
            self.board = board;
            None
          }
          GameResult::IllegalMove(_) => {
            return Err(ContractError::InvalidMove{});
          }
          GameResult::Stalemate => Some(CwChessResult::Stalemate),
          GameResult::Victory(color) => {
            match color {
              Color::Black => Some(CwChessResult::BlackCheckmates),
              Color::White => Some(CwChessResult::WhiteCheckmates)
            }
          }
        };
        Ok(self.result.clone())
      }
      _ => Err(ContractError::InvalidMove{})
    }
  }

  fn resign(&mut self) -> Result<Option<CwChessResult>, ContractError> {
    self.result = match self.side_to_move() {
      Color::Black => Some(CwChessResult::BlackResigns),
      Color::White => Some(CwChessResult::BlackResigns),
    };
    Ok(self.result.clone())
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
    pub result: Option<CwChessResult>,
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
        let mut game: Game = Game::new();
        for chess_move in self.moves.clone() {
            game.make_move(&chess_move)?;
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
            game.make_move(&chess_move)?;
            // save move
            self.moves.push(chess_move);
            // update result in case game ended
            self.result = game.result;
            Ok(self.result.clone())
        }
    }
}
