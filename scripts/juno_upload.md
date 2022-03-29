# Notes from deploying junochess 0.3.0

- use junod 2.1.0 in a container

```bash
docker run --rm -it --platform linux/amd64 -v "$(pwd)/artifacts:/artifacts:ro" ghcr.io/cosmoscontracts/juno:v2.1.0 /bin/sh
```

- configure environment

```sh
RPC="https://rpc.juno.omniflix.co:443"
CHAIN_ID="juno-1"
GAS_PRICES="0.025ujuno"
export NODE="${RPC}"
export TXFLAG="--node ${NODE}  --chain-id ${CHAIN_ID} --gas-prices ${GAS_PRICES} --gas auto --gas-adjustment 1.3"
```

- setup `chess` key with at least 1 JUNO

```sh
junod keys add chess --recover
# ...
junod query bank balances "addr"
# ...
```

> 0.3.3 is 272K.
> Wasn't sure on store cost since gas estimate != JUNO cost, so I had 3 JUNO to be safe.

- store wasm

```sh
junod tx wasm store /artifacts/0.3.0/cosmwasm_chess.wasm --from chess $TXFLAG
```

> - code_id `165`
> - 0.934757 JUNO
> - https://www.mintscan.io/juno/txs/35B2F0051CC337B72F48AB85B560C36C1885DD94B4CEB7720CCF9FC162BB4E0A

- instantiate contract

```sh
junod tx wasm instantiate 165 '{}' --from chess --label "junochess 0.3.0" $TXFLAG --admin juno1qzmu3y33vhwhexwwtctp7e3fn20qnfphy3f04w
```

> - contract address `juno1dk8kw6dwh6ugejuvkzumteu7pkj5vjkdl7pm8qwk5qtz7k2fh0kqce0s6f`
> - https://www.mintscan.io/juno/txs/72C721AA042FC7BA9AC0932FBD8F63CE8AAAF6112417DC1C9C203E3BD3D3A1DB
