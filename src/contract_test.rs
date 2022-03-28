#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::cwchess::{CwChessAction, CwChessColor, CwChessGame, CwChessGameOver, CwChessMove};
    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, GameSummary, InstantiateMsg, QueryMsg};

    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_binary, Env};

    #[test]
    fn test_initialize() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
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
    fn test_draw() {
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
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        )
        .unwrap();

        // cannot accept draw if not offered yet
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::AcceptDraw {},
                game_id: 1,
            },
        );
        match response.unwrap_err() {
            ContractError::InvalidMove { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // white offers draw
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::OfferDraw("d4".to_string()),
                game_id: 1,
            },
        )
        .unwrap();

        // black accepts
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("black", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::AcceptDraw {},
                game_id: 1,
            },
        )
        .unwrap();

        let game = from_binary::<CwChessGame>(
            &query(deps.as_ref(), mock_env(), QueryMsg::GetGame { game_id: 1 }).unwrap(),
        )
        .unwrap();
        assert_eq!(game.status, Some(CwChessGameOver::DrawAccepted {}));
    }

    #[test]
    fn test_get_games() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();

        // create first game (with two as white, one as black)
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("one", &[]),
            ExecuteMsg::CreateChallenge {
                block_time_limit: None,
                opponent: None,
                play_as: Some(CwChessColor::Black),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("two", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        )
        .unwrap();

        // get_games should return the game
        let games = from_binary::<Vec<GameSummary>>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetGames {
                    after: None,
                    game_over: None,
                    player: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].turn_color, Some(CwChessColor::White));

        // create second game (with one as white, two as black)
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("two", &[]),
            ExecuteMsg::CreateChallenge {
                block_time_limit: None,
                opponent: None,
                // creator is black
                play_as: Some(CwChessColor::Black),
            },
        )
        .unwrap();
        // opponent can accept
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("one", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 2 },
        )
        .unwrap();

        // white makes move in first game
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("two", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d4".to_string()),
                game_id: 1,
            },
        )
        .unwrap();

        // get games
        // internally player1 index is scanned before player2 index
        // this should still return games in order by game_id
        let games = from_binary::<Vec<GameSummary>>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetGames {
                    after: None,
                    game_over: None,
                    player: Some("one".to_string()),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].game_id, 1);
        assert_eq!(games[0].turn_color, Some(CwChessColor::Black));
        assert_eq!(games[0].player1, "two");
        assert_eq!(games[0].player2, "one");
        assert_eq!(games[1].game_id, 2);
        assert_eq!(games[1].turn_color, Some(CwChessColor::White));
        assert_eq!(games[1].player1, "one");
        assert_eq!(games[1].player2, "two");
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

    #[test]
    fn test_resign() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();
        // create game
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
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        )
        .unwrap();

        // check game status
        let game = from_binary::<CwChessGame>(
            &query(deps.as_ref(), mock_env(), QueryMsg::GetGame { game_id: 1 }).unwrap(),
        )
        .unwrap();
        assert_eq!(game.player1, "white");
        assert_eq!(game.player2, "black");
        assert_eq!(game.status, None);
        assert_eq!(game.turn_color(), Some(CwChessColor::White));

        // white resigns
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::Resign {},
                game_id: 1,
            },
        )
        .unwrap();

        // game status updated
        let game = from_binary::<CwChessGame>(
            &query(deps.as_ref(), mock_env(), QueryMsg::GetGame { game_id: 1 }).unwrap(),
        )
        .unwrap();
        assert_eq!(game.status, Some(CwChessGameOver::WhiteResigns));

        // cannot move after game over
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d4".to_string()),
                game_id: 1,
            },
        );
        match response.unwrap_err() {
            ContractError::GameAlreadyOver { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }
    }

    fn block_env(block: u64) -> Env {
        let mut env = mock_env();
        env.block.height = block;
        env
    }

    #[test]
    fn test_timeout() {
        let mut deps = mock_dependencies();

        // initialize
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {},
        )
        .unwrap();
        // create game with timeout
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("black", &[]),
            ExecuteMsg::CreateChallenge {
                // 300 blocks/per person @ ~10 blocks/minute => ~30 minutes/person
                block_time_limit: Some(300),
                opponent: None,
                // creator is black
                play_as: Some(CwChessColor::Black),
            },
        )
        .unwrap();
        // game created at block 100
        execute(
            deps.as_mut(),
            block_env(100),
            mock_info("white", &[]),
            ExecuteMsg::AcceptChallenge { challenge_id: 1 },
        )
        .unwrap();

        // first move, time limit starts
        execute(
            deps.as_mut(),
            block_env(300),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::from("d4"),
                game_id: 1,
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            block_env(310),
            mock_info("black", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("d5".to_string()),
                game_id: 1,
            },
        )
        .unwrap();

        // not a timeout yet (time starts after first move)
        let response = execute(
            deps.as_mut(),
            block_env(500),
            mock_info("black", &[]),
            ExecuteMsg::DeclareTimeout { game_id: 1 },
        );
        match response.unwrap_err() {
            ContractError::GameNotTimedOut { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        execute(
            deps.as_mut(),
            block_env(600),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::from("c4"),
                game_id: 1,
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            block_env(610),
            mock_info("black", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::MakeMove("dxc4".to_string()),
                game_id: 1,
            },
        )
        .unwrap();
        // white timed out
        let result = execute(
            deps.as_mut(),
            block_env(631),
            mock_info("white", &[]),
            ExecuteMsg::Move {
                action: CwChessAction::from("e3"),
                game_id: 1,
            },
        )
        .unwrap();
        assert_eq!(result.attributes[0].key, "game");
        assert_eq!(result.attributes[0].value.contains("white_timeout"), true);
    }
}
