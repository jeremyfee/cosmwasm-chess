#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessMove};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GameSummary, InstantiateMsg, QueryMsg};
use crate::state::{
    get_challenges_map, get_games_map, merge_iters, next_challenge_id, next_game_id, Challenge,
    State, STATE,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-chess";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    let height = env.block.height;
    let sender = info.sender;
    match msg {
        ExecuteMsg::CreateChallenge {
            opponent,
            play_as,
            block_time_limit,
        } => execute_create_challenge(deps, sender, opponent, play_as, block_time_limit, height),
        ExecuteMsg::AcceptChallenge { challenge_id } => {
            execute_accept_challenge(deps, challenge_id, sender, height)
        }
        ExecuteMsg::CancelChallenge { challenge_id } => {
            execute_cancel_challenge(deps, challenge_id, sender)
        }
        ExecuteMsg::Move { action, game_id } => execute_move(deps, game_id, sender, action, height),
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
    challenge_id: u64,
    player: Addr,
    start_height: u64,
) -> Result<Response, ContractError> {
    let challenges_map = get_challenges_map();
    let challenge = match challenges_map.load(deps.storage, challenge_id) {
        Ok(challenge) => {
            if challenge.opponent.is_some() && challenge.opponent != Some(player.clone()) {
                return Err(ContractError::NotYourChallenge {});
            }
            if challenge.created_by == player {
                return Err(ContractError::CannotPlaySelf {});
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
        challenge.play_as.clone(),
        start_height,
    );
    // create game
    let game = CwChessGame {
        block_time_limit: challenge.block_time_limit,
        game_id,
        player1: player1.clone(),
        player2: player2.clone(),
        moves: vec![],
        start_height,
        status: None,
        turn_color: Some(CwChessColor::White),
    };
    // update storage
    let games_map = get_games_map();
    games_map.save(deps.storage, game_id, &game)?;
    challenges_map.remove(deps.storage, challenge_id)?;

    Ok(Response::new()
        .add_attribute("accept_challenge", challenge_id.to_string())
        .add_attribute("game_id", game_id.to_string())
        .add_attribute("player1", player1)
        .add_attribute("player2", player2))
}

fn execute_cancel_challenge(
    deps: DepsMut,
    challenge_id: u64,
    player: Addr,
) -> Result<Response, ContractError> {
    let challenges_map = get_challenges_map();
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

    Ok(Response::new().add_attribute("cancel_challenge", challenge_id.to_string()))
}

fn execute_create_challenge(
    deps: DepsMut,
    created_by: Addr,
    opponent: Option<String>,
    play_as: Option<CwChessColor>,
    block_time_limit: Option<u64>,
    created_block: u64,
) -> Result<Response, ContractError> {
    let challenge_id = next_challenge_id(deps.storage)?;
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
        block_time_limit,
        challenge_id,
        created_block,
        created_by: created_by.clone(),
        opponent: opponent.clone(),
        play_as,
    };
    let challenges_map = get_challenges_map();
    challenges_map.save(deps.storage, challenge_id, &challenge)?;

    Ok(Response::new()
        .add_attribute("create_challenge", challenge_id.to_string())
        .add_attribute("created_by", created_by)
        .add_attribute(
            "opponent",
            opponent.unwrap_or_else(|| Addr::unchecked("none")),
        ))
}

fn execute_move(
    deps: DepsMut,
    game_id: u64,
    player: Addr,
    action: CwChessAction,
    height: u64,
) -> Result<Response, ContractError> {
    let games_map = get_games_map();
    let game = games_map.update(deps.storage, game_id, |game| -> Result<_, ContractError> {
        match game {
            None => Err(ContractError::GameNotFound {}),
            Some(mut game) => {
                game.make_move(
                    &player,
                    CwChessMove {
                        action: action.clone(),
                        block: height,
                    },
                )?;
                Ok(game)
            }
        }
    })?;

    Ok(Response::new()
        .add_attribute("game_id", game.game_id.to_string())
        .add_attribute("move", format!("{:?}", action))
        .add_attribute("player1", game.player1)
        .add_attribute("player2", game.player2))
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
