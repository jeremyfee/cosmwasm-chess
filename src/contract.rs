#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessMove};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GameSummary, InstantiateMsg, QueryMsg};
use crate::state::{
    get_challenges_map, get_games_map, next_challenge_id, next_game_id, Challenge, State, STATE,
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
        } => try_create_challenge(deps, sender, opponent, play_as, block_time_limit, height),
        ExecuteMsg::AcceptChallenge { challenge_id } => {
            try_accept_challenge(deps, challenge_id, sender, height)
        }
        ExecuteMsg::CancelChallenge { challenge_id } => {
            try_cancel_challenge(deps, challenge_id, sender)
        }
        ExecuteMsg::Move { action, game_id } => try_move(deps, game_id, sender, action, height),
    }
}

fn try_accept_challenge(
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
        player1,
        player2,
        moves: vec![],
        start_height,
        status: None,
        turn_color: Some(CwChessColor::White),
    };
    // update storage
    let games_map = get_games_map();
    games_map.save(deps.storage, game_id, &game)?;
    challenges_map.remove(deps.storage, challenge_id)?;
    Ok(Response::new().add_attribute("game_id", game_id.to_string()))
}

fn try_cancel_challenge(
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
    Ok(Response::new())
}

fn try_create_challenge(
    deps: DepsMut,
    created_by: Addr,
    opponent: Option<String>,
    play_as: Option<CwChessColor>,
    block_time_limit: Option<u64>,
    created_block: u64,
) -> Result<Response, ContractError> {
    let challenge_id = next_challenge_id(deps.storage)?;
    let opponent = match opponent {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };
    let challenge = Challenge {
        block_time_limit,
        challenge_id,
        created_block,
        created_by,
        opponent,
        play_as,
    };
    let challenges_map = get_challenges_map();
    challenges_map.save(deps.storage, challenge_id, &challenge)?;
    Ok(Response::new().add_attribute("challenge_id", challenge_id.to_string()))
}

fn try_move(
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
        .add_attribute("action", format!("{:?}", action)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetGame { game_id } => to_binary(&query_get_game(deps, game_id)?),
        QueryMsg::GetChallenge { challenge_id } => {
            to_binary(&query_get_challenge(deps, challenge_id)?)
        }
        QueryMsg::GetChallenges { player } => to_binary(&query_get_challenges(deps, player)?),
        QueryMsg::GetGames { game_over, player } => {
            to_binary(&query_get_games(deps, game_over, player)?)
        }
    }
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

fn query_get_challenges(deps: Deps, player: Option<String>) -> StdResult<Vec<Challenge>> {
    let mut challenges: Vec<Challenge> = vec![];
    let challenges_map = get_challenges_map();

    if let Some(addr) = player {
        let addr = deps.api.addr_validate(&addr)?;
        challenges.extend(
            challenges_map
                .idx
                .created_by
                .prefix(addr.clone())
                .range(deps.storage, None, None, Order::Ascending)
                .map(|challenge| -> Challenge {
                    let (_, c) = challenge.unwrap();
                    c
                }),
        );
        challenges.extend(
            challenges_map
                .idx
                .opponent
                .prefix(addr)
                .range(deps.storage, None, None, Order::Ascending)
                .map(|challenge| -> Challenge {
                    let (_, c) = challenge.unwrap();
                    c
                }),
        );
    } else {
        challenges.extend(
            challenges_map
                .idx
                .opponent
                .prefix(Addr::unchecked("none"))
                .range(deps.storage, None, None, Order::Ascending)
                .map(|challenge| -> Challenge {
                    let (_, c) = challenge.unwrap();
                    c
                }),
        );
    }
    Ok(challenges)
}

fn query_get_games(
    deps: Deps,
    game_over: Option<bool>,
    player: Option<String>,
) -> StdResult<Vec<GameSummary>> {
    let mut games = vec![];
    let games_map = get_games_map();
    let game_over = game_over.unwrap_or(false);
    if let Some(addr) = player {
        let addr = deps.api.addr_validate(&addr)?;
        games.extend(
            games_map
                .idx
                .player1
                .prefix(addr.clone())
                .range(deps.storage, None, None, Order::Ascending)
                .map(|game| -> GameSummary {
                    let (_, g) = game.unwrap();
                    GameSummary::from(&g)
                })
                // filter games that are over unless requested
                .filter(|s| -> bool { game_over || s.status.is_none() }),
        );
        games.extend(
            games_map
                .idx
                .player2
                .prefix(addr)
                .range(deps.storage, None, None, Order::Ascending)
                .map(|game| -> GameSummary {
                    let (_, g) = game.unwrap();
                    GameSummary::from(&g)
                })
                // filter games that are over unless requested
                .filter(|s| -> bool { game_over || s.status.is_none() }),
        );
    } else {
        games.extend(
            games_map
                .range(deps.storage, None, None, Order::Ascending)
                .map(|game| -> GameSummary {
                    let (_, g) = game.unwrap();
                    GameSummary::from(&g)
                })
                // filter games that are over unless requested
                .filter(|s| -> bool { game_over || s.status.is_none() })
                // limit non-player specific requests to
                .take(100),
        );
    }

    Ok(games)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create_challenge() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let mut env = mock_env();
        env.block.height = 123;
        let info = mock_info("owner", &coins(1000, "hello"));
        let _contract_addr = env.clone().contract.address;
        let init_res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        // create a challenge with an opponent
        let msg = ExecuteMsg::CreateChallenge {
            block_time_limit: None,
            opponent: Some("opponent".to_string()),
            play_as: None,
        };
        let mut env = mock_env();
        env.block.height = 456;
        let info = mock_info("creator", &[]);
        let execute_res = execute(deps.as_mut(), env, info, msg);
        let attr = execute_res.unwrap().attributes[0].clone();
        assert_eq!(&attr.key, "challenge_id");
        assert_eq!(&attr.value, "1");
    }

    #[test]
    fn test_accept_challenge_open() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();
        // create challenge
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::CreateChallenge {
                block_time_limit: None,
                opponent: None,
                play_as: None,
            },
        )
        .unwrap();

        // can accept open challenge
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("other", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        );
        let attr = response.unwrap().attributes[0].clone();
        assert_eq!(&attr.key, "game_id");
        assert_eq!(&attr.value, "1");

        // not found after accepted
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("other", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        );
        match response.unwrap_err() {
            ContractError::ChallengeNotFound { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_accept_challenge_specific_opponent() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();
        // create challenge
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::CreateChallenge {
                block_time_limit: None,
                opponent: Some("opponent".to_string()),
                play_as: None,
            },
        )
        .unwrap();

        // cannot accept if not opponent
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("other", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        );
        match response.unwrap_err() {
            ContractError::NotYourChallenge { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // opponent can accept
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("opponent", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        );
        let attr = response.unwrap().attributes[0].clone();
        assert_eq!(&attr.key, "game_id");
        assert_eq!(&attr.value, "1");
    }

    #[test]
    fn test_make_move() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();
        // create challenge
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("black", &[]),
            ExecuteMsg::CreateChallenge {
                block_time_limit: None,
                opponent: None,
                // creator is black
                play_as: Some(CwChessColor::Black),
            },
        )
        .unwrap();
        // opponent can accept
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        );
        let attr = response.unwrap().attributes[0].clone();
        assert_eq!(&attr.key, "game_id");
        assert_eq!(&attr.value, "1");

        // first move by white
        let mut env = mock_env();
        env.block.height = 123;
        execute(
            deps.as_mut(),
            env,
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d4".to_string()),
                game_id: 1,
            },
        )
        .unwrap();

        // white cannot go when blacks turn
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("c4".to_string()),
                game_id: 1,
            },
        );
        match response.unwrap_err() {
            ContractError::NotYourTurn { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // first move by black
        let mut env = mock_env();
        env.block.height = 456;
        execute(
            deps.as_mut(),
            env,
            mock_info("black", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d5".to_string()),
                game_id: 1,
            },
        )
        .unwrap();

        // check in on game status
        let game = from_binary::<CwChessGame>(
            &query(deps.as_ref(), mock_env(), QueryMsg::GetGame { game_id: 1 }).unwrap(),
        )
        .unwrap();
        assert_eq!(
            game.moves,
            vec![
                CwChessMove {
                    action: CwChessAction::MakeMove("d4".to_string()),
                    block: 123
                },
                CwChessMove {
                    action: CwChessAction::MakeMove("d5".to_string()),
                    block: 456
                },
            ]
        );

        // white cannot make invalid move (pawn already there)
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d5".to_string()),
                game_id: 1,
            },
        );
        match response.unwrap_err() {
            ContractError::InvalidMove { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // white can make a valid move
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("c4".to_string()),
                game_id: 1,
            },
        )
        .unwrap();
    }
}
