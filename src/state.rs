use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cwchess::{CwChessColor, CwChessGame};
use cosmwasm_std::{Addr, StdError, StdResult, Storage};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Challenge {
    pub block_time_limit: Option<u64>,
    pub challenge_id: u64,
    pub created_block: u64,
    pub created_by: Addr,
    pub play_as: Option<CwChessColor>,
    pub opponent: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Player {
    challenges: Vec<u64>,
    games: Vec<u64>,
}

pub const CHALLENGE_ID: Item<u64> = Item::new("challenge_id");
pub const CHALLENGES: Map<u64, Challenge> = Map::new("challenges");
pub const GAME_ID: Item<u64> = Item::new("game_id");
pub const GAMES: Map<u64, CwChessGame> = Map::new("games");
pub const PLAYERS: Map<&Addr, Player> = Map::new("players");
pub const STATE: Item<State> = Item::new("state");

pub fn next_challenge_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = CHALLENGE_ID.may_load(store)?.unwrap_or_default() + 1;
    CHALLENGE_ID.save(store, &id)?;
    Ok(id)
}

pub fn next_game_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = GAME_ID.may_load(store)?.unwrap_or_default() + 1;
    GAME_ID.save(store, &id)?;
    Ok(id)
}

pub fn add_challenge(store: &mut dyn Storage, challenge: Challenge) -> StdResult<Challenge> {
    CHALLENGES.save(store, challenge.challenge_id, &challenge)?;
    add_player_challenge(store, &challenge.created_by, challenge.challenge_id)?;
    if let Some(addr) = challenge.opponent.clone() {
        add_player_challenge(store, &addr, challenge.challenge_id)?;
    }
    Ok(challenge)
}

pub fn remove_challenge(store: &mut dyn Storage, challenge: Challenge) -> StdResult<Challenge> {
    CHALLENGES.remove(store, challenge.challenge_id);
    remove_player_challenge(store, &challenge.created_by, challenge.challenge_id)?;
    if let Some(addr) = challenge.opponent.clone() {
        remove_player_challenge(store, &addr, challenge.challenge_id)?;
    }
    Ok(challenge)
}

pub fn add_game(store: &mut dyn Storage, game: CwChessGame) -> StdResult<CwChessGame> {
    GAMES.save(store, game.game_id, &game)?;
    add_player_game(store, &game.player1, game.game_id)?;
    add_player_game(store, &game.player2, game.game_id)?;
    Ok(game)
}

fn add_player_challenge(
    store: &mut dyn Storage,
    addr: &Addr,
    challenge_id: u64,
) -> StdResult<Player> {
    PLAYERS.update(store, addr, |player| -> StdResult<Player> {
        match player {
            Some(mut player) => {
                player.challenges.push(challenge_id);
                Ok(player)
            }
            None => Ok(Player {
                challenges: vec![challenge_id],
                games: vec![],
            }),
        }
    })
}

fn remove_player_challenge(
    store: &mut dyn Storage,
    addr: &Addr,
    challenge_id: u64,
) -> StdResult<Player> {
    let player = PLAYERS.update(store, addr, |player| -> StdResult<Player> {
        match player {
            Some(mut player) => {
                if let Some(index) = player.challenges.iter().position(|&c| c == challenge_id) {
                    player.challenges.remove(index);
                }
                Ok(player)
            }
            None => Err(StdError::NotFound {
                kind: "Player".to_string(),
            }),
        }
    })?;
    // clean up empty players
    if player.challenges.len() == 0 && player.games.len() == 0 {
        PLAYERS.remove(store, addr);
    }
    Ok(player)
}

fn add_player_game(store: &mut dyn Storage, addr: &Addr, game_id: u64) -> StdResult<Player> {
    PLAYERS.update(store, addr, |player| -> StdResult<Player> {
        match player {
            Some(mut player) => {
                player.games.push(game_id);
                Ok(player)
            }
            None => Ok(Player {
                challenges: vec![],
                games: vec![game_id],
            }),
        }
    })
}
