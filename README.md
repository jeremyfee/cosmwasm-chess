# cosmwasm-chess

A CosmWasm smart contract for creating and playing chess games.

## Overview

Users interact with the smart contract to create challenges, accept/cancel an existing
challenge, and take turns in chess games.

Challenges can specify a specific color or choose "randomly", as well as choose a
specific opponent or remain open to any other player. A per-player block time limit
can also be used as a "clock". On each Game turn, users make a Move, make a move and
Offer a Draw, Accept a Draw offer, or Resign.

There are query methods to get multiple challenge or game summaries or individual
challenge or game details. Summary queries are limited to keep result sizes managable
and support an "after" parameter for paging results.

## Deployment

- `v0.4.1`

  ```
  juno19jrfw6y7ljxnh389cl9eewrs4rfgf0w92g0m59lp0llvdsf8a0csunfq3p
  ```

## Development Notes

Using `cw-storage-plus` indexed map to store challenges and games, and maintain indexes
based on player addresses used for queries.

I originally implemented using the `chess` crate, but the resulting WASM was over 1MB!
I found a lighter weight crate `chess-engine` missing a few features that brought WASM
size to a few hundred KB. Currently using a fork with bug fixes and new features until
those changes can be merged into the upstream.

https://github.com/jeremyfee/chess-engine/tree/game

The original implementation stored moves and re-executed the move on every contract
execution, in an attempt to reduce storage usage. This caused the gas usage to increase
significantly with each move (starting under 200k/move, and over 500k/move after 80
moves). v0.4.0 uses a FEN string to store/load board state, which is more efficient,
and gas usage now remains under 300k/move even for long games.

### Local Testing

There are several scripts in the `scripts` directory to run the contract on a local
node for testing.

```
cargo wasm
scripts/local_deploy.sh | tee junod_env.sh
source junod_env.sh

junod_execute `{"create_challenge": {}}` --from test-user
junod_execute `{"accept_challenge": {"challenge_id": 1}}' --from test-user2
junod_query `{"get_games":{}}`
```

## UI

Separate project: https://github.com/jeremyfee/cosmwasm-chess-ui
