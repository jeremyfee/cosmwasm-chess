#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessMove};
    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
        assert_eq!(&attr.key, "create_challenge");
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
        let attrs = response.unwrap().attributes;
        let attr = attrs[0].clone();
        assert_eq!(&attr.key, "accept_challenge");
        assert_eq!(&attr.value, "1");
        let attr = attrs[1].clone();
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
        let attrs = response.unwrap().attributes;
        let attr = attrs[0].clone();
        assert_eq!(&attr.key, "accept_challenge");
        assert_eq!(&attr.value, "1");
        let attr = attrs[1].clone();
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
        let attrs = response.unwrap().attributes;
        let attr = attrs[0].clone();
        assert_eq!(&attr.key, "accept_challenge");
        assert_eq!(&attr.value, "1");
        let attr = attrs[1].clone();
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
