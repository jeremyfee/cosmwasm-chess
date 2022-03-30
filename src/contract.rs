#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GameSummary, InstantiateMsg, QueryMsg};
use crate::state::{
    get_challenges_map, get_games_map, merge_iters, next_challenge_id, next_game_id, Challenge,
    State, STATE,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-chess";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AcceptChallenge { challenge_id } => {
            execute_accept_challenge(deps, env, info, challenge_id)
        }
        ExecuteMsg::CancelChallenge { challenge_id } => {
            execute_cancel_challenge(deps, info, challenge_id)
        }
        ExecuteMsg::CreateChallenge {
            block_limit,
            opponent,
            play_as,
        } => execute_create_challenge(deps, env, info, block_limit, opponent, play_as),
        ExecuteMsg::DeclareTimeout { game_id } => execute_declare_timeout(deps, env, game_id),
        ExecuteMsg::Turn { action, game_id } => execute_turn(deps, env, info, action, game_id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetGame { game_id } => to_binary(&query_get_game(deps, game_id)?),
        QueryMsg::GetChallenge { challenge_id } => {
            to_binary(&query_get_challenge(deps, challenge_id)?)
        }
        QueryMsg::GetChallenges { after, player } => {
            to_binary(&query_get_challenges(deps, after, player)?)
        }
        QueryMsg::GetGames {
            after,
            game_over,
            player,
        } => to_binary(&query_get_games(deps, after, game_over, player)?),
    }
}

fn execute_accept_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    challenge_id: u64,
) -> Result<Response, ContractError> {
    let block_start = env.block.height;
    let challenges_map = get_challenges_map();
    let player = info.sender;
    // find challenge
    let challenge = match challenges_map.load(deps.storage, challenge_id) {
        Ok(challenge) => {
            if challenge.created_by == player {
                return Err(ContractError::CannotPlaySelf {});
            }
            if let Some(opponent) = challenge.opponent.clone() {
                if opponent != player {
                    return Err(ContractError::NotYourChallenge {});
                }
            }
            challenge
        }
        _ => {
            return Err(ContractError::ChallengeNotFound {});
        }
    };
    // create game
    let game_id = next_game_id(deps.storage)?;
    let (player1, player2) = CwChessGame::get_player_order(
        challenge.created_by.clone(),
        player,
        challenge.play_as,
        block_start,
    );
    // create game
    let game = CwChessGame {
        block_limit: challenge.block_limit,
        block_start,
        fen: DEFAULT_FEN.to_string(),
        game_id,
        player1: player1.clone(),
        player2: player2.clone(),
        moves: vec![],
        status: None,
    };
    // update storage
    let games_map = get_games_map();
    games_map.save(deps.storage, game_id, &game)?;
    challenges_map.remove(deps.storage, challenge_id)?;

    Ok(Response::new()
        .add_attribute("action", "accept_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("game_id", game_id.to_string())
        .add_attribute("player1", player1)
        .add_attribute("player2", player2))
}

fn execute_cancel_challenge(
    deps: DepsMut,
    info: MessageInfo,
    challenge_id: u64,
) -> Result<Response, ContractError> {
    let challenges_map = get_challenges_map();
    let player = info.sender;
    let challenge = match challenges_map.load(deps.storage, challenge_id) {
        Ok(challenge) => {
            if challenge.created_by != player {
                return Err(ContractError::NotYourChallenge {});
            }
            challenge
        }
        _ => {
            return Err(ContractError::ChallengeNotFound {});
        }
    };
    challenges_map.remove(deps.storage, challenge.challenge_id)?;

    Ok(Response::new()
        .add_attribute("action", "cancel_challenge")
        .add_attribute("challenge_id", challenge_id.to_string()))
}

fn execute_create_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    block_limit: Option<u64>,
    opponent: Option<String>,
    play_as: Option<CwChessColor>,
) -> Result<Response, ContractError> {
    let block_created = env.block.height;
    let challenge_id = next_challenge_id(deps.storage)?;
    let created_by = info.sender;
    let opponent = match opponent {
        Some(addr) => {
            let addr = deps.api.addr_validate(&addr)?;
            if created_by == addr {
                return Err(ContractError::CannotPlaySelf {});
            }
            Some(addr)
        }
        None => None,
    };
    let challenge = Challenge {
        block_created,
        block_limit,
        challenge_id,
        created_by: created_by.clone(),
        opponent: opponent.clone(),
        play_as,
    };
    let challenges_map = get_challenges_map();
    challenges_map.save(deps.storage, challenge_id, &challenge)?;

    Ok(Response::new()
        .add_attribute("action", "create_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("created_by", created_by))
}

fn execute_declare_timeout(
    deps: DepsMut,
    env: Env,
    game_id: u64,
) -> Result<Response, ContractError> {
    let games_map = get_games_map();
    let height = env.block.height;
    let game = games_map.update(deps.storage, game_id, |game| -> Result<_, ContractError> {
        match game {
            None => Err(ContractError::GameNotFound {}),
            Some(mut game) => match game.check_timeout(height)? {
                None => Err(ContractError::GameNotTimedOut {}),
                _ => Ok(game),
            },
        }
    })?;

    Ok(Response::new()
        .add_attribute("action", "declare_timeout")
        .add_attribute("game_id", game.game_id.to_string()))
}

fn execute_turn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: CwChessAction,
    game_id: u64,
) -> Result<Response, ContractError> {
    let games_map = get_games_map();
    let height = env.block.height;
    let player = info.sender;
    let game = games_map.update(deps.storage, game_id, |game| -> Result<_, ContractError> {
        match game {
            None => Err(ContractError::GameNotFound {}),
            Some(mut game) => {
                game.make_move(&player, (height, action.clone()))?;
                Ok(game)
            }
        }
    })?;

    Ok(Response::new()
        .add_attribute("action", "turn")
        .add_attribute("game_id", game.game_id.to_string())
        .add_attribute(
            "status",
            game.status
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| format!("{:?}", game.turn_color())),
        ))
}

fn query_get_challenge(deps: Deps, challenge_id: u64) -> StdResult<Challenge> {
    let challenges_map = get_challenges_map();
    let challenge = challenges_map.load(deps.storage, challenge_id)?;

    Ok(challenge)
}

fn query_get_game(deps: Deps, game_id: u64) -> StdResult<CwChessGame> {
    let games_map = get_games_map();
    let game = games_map.load(deps.storage, game_id)?;

    Ok(game)
}

fn query_get_challenges(
    deps: Deps,
    after: Option<u64>,
    player: Option<String>,
) -> StdResult<Vec<Challenge>> {
    let challenges_map = get_challenges_map();
    let after = after.map(Bound::exclusive);

    let challenges = match player {
        None => {
            let open_challenges = challenges_map
                .idx
                .opponent
                .prefix(Addr::unchecked("none"))
                .range(deps.storage, after, None, Order::Ascending)
                .map(|result| -> Challenge { result.unwrap().1 });

            open_challenges.take(25).collect::<Vec<_>>()
        }
        Some(addr) => {
            let addr = deps.api.addr_validate(&addr)?;
            let created_by = challenges_map
                .idx
                .created_by
                .prefix(addr.clone())
                .range(deps.storage, after.clone(), None, Order::Ascending)
                .map(|result| -> Challenge { result.unwrap().1 });
            let opponent = challenges_map
                .idx
                .opponent
                .prefix(addr)
                .range(deps.storage, after, None, Order::Ascending)
                .map(|result| -> Challenge { result.unwrap().1 });

            merge_iters(created_by, opponent, |c1, c2| -> bool {
                c1.challenge_id <= c2.challenge_id
            })
            .take(25)
            .collect::<Vec<_>>()
        }
    };

    Ok(challenges)
}

fn query_get_games(
    deps: Deps,
    after: Option<u64>,
    game_over: Option<bool>,
    player: Option<String>,
) -> StdResult<Vec<GameSummary>> {
    let games_map = get_games_map();
    let after = after.map(Bound::exclusive);
    let game_over = game_over.unwrap_or(false);

    let games = match player {
        None => {
            let all_games = games_map
                .range(deps.storage, after, None, Order::Ascending)
                .map(|result| -> CwChessGame { result.unwrap().1 });

            all_games
                .filter(|g| -> bool { game_over || g.status.is_none() })
                .map(|game| -> GameSummary { GameSummary::from(&game) })
                .take(25)
                .collect::<Vec<_>>()
        }
        Some(addr) => {
            let addr = deps.api.addr_validate(&addr)?;
            let player1 = games_map
                .idx
                .player1
                .prefix(addr.clone())
                .range(deps.storage, after.clone(), None, Order::Ascending)
                .map(|result| -> CwChessGame { result.unwrap().1 });
            let player2 = games_map
                .idx
                .player2
                .prefix(addr)
                .range(deps.storage, after, None, Order::Ascending)
                .map(|result| -> CwChessGame { result.unwrap().1 });

            merge_iters(player1, player2, |g1, g2| -> bool {
                g1.game_id <= g2.game_id
            })
            .filter(|g| -> bool { game_over || g.status.is_none() })
            .map(|game| -> GameSummary { GameSummary::from(&game) })
            .take(25)
            .collect::<Vec<_>>()
        }
    };

    Ok(games)
}
