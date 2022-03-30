use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::iter::Peekable;

use crate::cwchess::{CwChessColor, CwChessGame};

// STATE

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub owner: Addr,
}

pub const STATE: Item<State> = Item::new("state");

// CHALLENGES

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Challenge {
    pub block_created: u64,
    pub block_limit: Option<u64>,
    pub challenge_id: u64,
    pub created_by: Addr,
    pub play_as: Option<CwChessColor>,
    pub opponent: Option<Addr>,
}

pub const CHALLENGE_ID: Item<u64> = Item::new("challenge_id");

pub fn next_challenge_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = CHALLENGE_ID.may_load(store)?.unwrap_or_default() + 1;
    CHALLENGE_ID.save(store, &id)?;
    Ok(id)
}

pub struct ChallengeIndexes<'a> {
    pub created_by: MultiIndex<'a, Addr, Challenge, u64>,
    pub opponent: MultiIndex<'a, Addr, Challenge, u64>,
}

impl<'a> IndexList<Challenge> for ChallengeIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Challenge>> + '_> {
        let v: Vec<&dyn Index<Challenge>> = vec![&self.created_by, &self.opponent];
        Box::new(v.into_iter())
    }
}

pub fn get_challenges_map<'a>() -> IndexedMap<'a, u64, Challenge, ChallengeIndexes<'a>> {
    let indexes = ChallengeIndexes {
        created_by: MultiIndex::new(
            |c: &Challenge| c.created_by.clone(),
            "challenges",
            "challenges__created_by",
        ),
        opponent: MultiIndex::new(
            |c: &Challenge| {
                c.opponent
                    .clone()
                    .unwrap_or_else(|| Addr::unchecked("none"))
            },
            "challenges",
            "challenges__opponent",
        ),
    };
    IndexedMap::new("challenges", indexes)
}

// GAMES

pub const GAME_ID: Item<u64> = Item::new("game_id");

pub fn next_game_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = GAME_ID.may_load(store)?.unwrap_or_default() + 1;
    GAME_ID.save(store, &id)?;
    Ok(id)
}

pub struct GameIndexes<'a> {
    pub player1: MultiIndex<'a, Addr, CwChessGame, u64>,
    pub player2: MultiIndex<'a, Addr, CwChessGame, u64>,
}

impl<'a> IndexList<CwChessGame> for GameIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CwChessGame>> + '_> {
        let v: Vec<&dyn Index<CwChessGame>> = vec![&self.player1, &self.player2];
        Box::new(v.into_iter())
    }
}

pub fn get_games_map<'a>() -> IndexedMap<'a, u64, CwChessGame, GameIndexes<'a>> {
    let indexes = GameIndexes {
        player1: MultiIndex::new(
            |c: &CwChessGame| c.player1.clone(),
            "games",
            "games__player1",
        ),
        player2: MultiIndex::new(
            |c: &CwChessGame| c.player2.clone(),
            "games",
            "games__player2",
        ),
    };
    IndexedMap::new("games", indexes)
}

pub fn merge_iters<I, J, K>(
    iter1: I,
    iter2: J,
    is_less_than: fn(&I::Item, &J::Item) -> bool,
) -> IterMerge<I, J, K>
where
    I: Iterator<Item = K>,
    J: Iterator<Item = K>,
{
    IterMerge {
        iter1: iter1.peekable(),
        iter2: iter2.peekable(),
        is_less_than,
    }
}

/**
 * Utility to merge multiple index ranges.
 *
 * Inspired by itertools 0.10.0 merge_join_by.
 */
pub struct IterMerge<I, J, K>
where
    I: Iterator<Item = K>,
    J: Iterator<Item = K>,
{
    iter1: Peekable<I>,
    iter2: Peekable<J>,
    // return true to return first item, false for second item
    is_less_than: fn(&K, &K) -> bool,
}

impl<I, J, K> Iterator for IterMerge<I, J, K>
where
    I: Iterator<Item = K>,
    J: Iterator<Item = K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        let item1 = self.iter1.peek();
        let item2 = self.iter2.peek();
        match (item1, item2) {
            (None, None) => None,
            (Some(_), None) => self.iter1.next(),
            (None, Some(_)) => self.iter2.next(),
            (Some(item1), Some(item2)) => {
                let is_less_than = self.is_less_than;
                if is_less_than(item1, item2) {
                    self.iter1.next()
                } else {
                    self.iter2.next()
                }
            }
        }
    }
}
