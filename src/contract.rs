#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessMove, CwChessStatus};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    add_challenge, add_game, next_challenge_id, next_game_id, remove_challenge, Challenge, Player,
    State, CHALLENGES, GAMES, PLAYERS, STATE,
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
    match msg {
        ExecuteMsg::CreateChallenge {
            opponent,
            play_as,
            block_time_limit,
        } => try_create_challenge(deps, env, info, opponent, play_as, block_time_limit),
        ExecuteMsg::AcceptChallenge { challenge_id } => {
            try_accept_challenge(deps, challenge_id, info.sender, env.block.height)
        }
        ExecuteMsg::CancelChallenge { challenge_id } => {
            try_cancel_challenge(deps, challenge_id, info.sender)
        }
        ExecuteMsg::Move { action, game_id } => {
            try_move(deps, game_id, info.sender, action, env.block.height)
        }
    }
}

fn try_accept_challenge(
    deps: DepsMut,
    challenge_id: u64,
    player: Addr,
    height: u64,
) -> Result<Response, ContractError> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    if challenge.opponent.is_some() && challenge.opponent.clone() != Some(player.clone()) {
        return Err(ContractError::NotYourChallenge {});
    }
    let (player1, player2) = CwChessGame::get_player_order(
        challenge.created_by.clone(),
        player,
        challenge.play_as.clone(),
        height,
    );
    // create game
    let game_id = next_game_id(deps.storage)?;
    let game = CwChessGame {
        block_time_limit: challenge.block_time_limit,
        game_id: game_id,
        player1: player1.clone(),
        player2: player2.clone(),
        moves: vec![],
        status: CwChessStatus::Ongoing,
    };
    // update storage
    add_game(deps.storage, game)?;
    remove_challenge(deps.storage, challenge)?;
    Ok(Response::new())
}

fn try_cancel_challenge(
    deps: DepsMut,
    challenge_id: u64,
    player: Addr,
) -> Result<Response, ContractError> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    if challenge.created_by != player {
        return Err(ContractError::NotYourChallenge {});
    }
    CHALLENGES.remove(deps.storage, challenge_id);
    Ok(Response::new())
}

fn try_create_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    opponent: Option<String>,
    play_as: Option<CwChessColor>,
    block_time_limit: Option<u64>,
) -> Result<Response, ContractError> {
    let challenge_id = next_challenge_id(deps.storage)?;
    let opponent_addr = match opponent {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };
    let challenge = Challenge {
        block_time_limit: block_time_limit,
        challenge_id: challenge_id,
        created_block: env.block.height,
        created_by: info.sender.clone(),
        opponent: opponent_addr.clone(),
        play_as: play_as,
    };
    add_challenge(deps.storage, challenge)?;
    Ok(Response::new().add_attribute("challenge_id", challenge_id.to_string()))
}

pub fn try_move(
    deps: DepsMut,
    game_id: u64,
    player: Addr,
    action: CwChessAction,
    height: u64,
) -> Result<Response, ContractError> {
    let game = GAMES.update(deps.storage, game_id, |game| -> Result<_, ContractError> {
        match game {
            None => Err(ContractError::GameNotFound {}),
            Some(mut game) => {
                game.make_move(
                    &player,
                    CwChessMove {
                        action: action,
                        block: height,
                    },
                )?;
                Ok(game)
            }
        }
    })?;
    Ok(Response::new().add_attribute("game_id", game.game_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetGame { game_id } => to_binary(&query_get_game(deps, game_id)?),
        QueryMsg::GetChallenge { challenge_id } => {
            to_binary(&query_get_challenge(deps, challenge_id)?)
        }
        QueryMsg::GetPlayerInfo { player } => to_binary(&query_get_player_info(
            deps,
            deps.api.addr_validate(&player)?,
        )?),
    }
}

fn query_get_challenge(deps: Deps, challenge_id: u64) -> StdResult<Challenge> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    Ok(challenge)
}

fn query_get_game(deps: Deps, game_id: u64) -> StdResult<CwChessGame> {
    let game = GAMES.load(deps.storage, game_id)?;
    Ok(game)
}

fn query_get_player_info(deps: Deps, player: Addr) -> StdResult<Player> {
    let player = PLAYERS.load(deps.storage, &player)?;
    Ok(player)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
