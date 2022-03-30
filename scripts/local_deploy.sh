#! /bin/bash

###############################################################################
# start a local junod node in a docker container,
# upload and instantiate a contract,
###############################################################################

# contract config
CONTRACT_INSTANTIATE_MESSAGE=${CONTRACT_INSTANTIATE_MESSAGE:-'{}'};
CONTRACT_LABEL=${CONTRACT_LABEL:-"cosmwasm-chess"};
CONTRACT_WASM=${CONTRACT_WASM:-"../target/wasm32-unknown-unknown/release/cosmwasm_chess.wasm"};

# chain config
CHAIN_ID=${CHAIN_ID:-"testing"};
FEE_TOKEN=${FEE_TOKEN:-"ujuno"};
GAS=${GAS:-"auto"};
GAS_ADJUSTMENT=${GAS_ADJUSTMENT:-"1.3"};
GAS_LIMIT=${GAS_LIMIT:-"100000000"};
GAS_PRICES=${GAS_PRICES:-"0.01${FEE_TOKEN}"};
STAKE_TOKEN=${STAKE_TOKEN:-"ustake"};

# container config
CONTAINER_TAG=${CONTAINER_TAG:-"v2.3.0-beta.1"};
CONTAINER_IMAGE=${CONTAINER_IMAGE:-"ghcr.io/cosmoscontracts/juno:${CONTAINER_TAG}"};
CONTAINER_NAME=${CONTAINER_NAME:-"junod_local"};

###############################################################################

# work from script directory
cd "$(dirname "${0}")" || (echo "Unable to change directory"; exit 1);

# make sure jq is installed
if ! command -v jq 1>/dev/null; then
  echo "jq not found";
  echo "On a mac, try 'brew install jq'"
fi

# make sure contract wasm is built
if [ ! -f "${CONTRACT_WASM}" ]; then
  echo "Contract ${CONTRACT_WASM} not found";
  echo "Run 'cargo wasm' to build";
  exit 1;
fi

# make sure local node is running
if ! docker ps | grep -q "${CONTAINER_NAME}"; then
  echo "# Starting container '${CONTAINER_NAME}'";
  # start local container
  CONTAINER_ID=$( \
    docker run \
      -d \
      -e "FEE_TOKEN=${FEE_TOKEN}" \
      -e "GAS_LIMIT=${GAS_LIMIT}" \
      -e "STAKE_TOKEN=${STAKE_TOKEN}" \
      --name "${CONTAINER_NAME}" \
      -p 1317:1317 \
      -p 26656:26656 \
      -p 26657:26657 \
      --platform linux/amd64 \
      "${CONTAINER_IMAGE}" \
      "./setup_and_run.sh" \
      "juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y" \
      "juno102fjg5u62qkgsux9z9fl652mw8r98kgcgjv99m" \
  );
  if [ -z "${CONTAINER_ID}" ]; then
    echo "Error starting container, bailing";
    exit 1;
  fi
  # wait a bit for chain to bootstrap
  echo "# Waiting 10s for chain to start";
  sleep 10;
else
  echo "# Using existing container '${CONTAINER_NAME}'";
fi

# internal configuration
DOCKER_EXEC="docker exec -it ${CONTAINER_NAME}";
QUERY_ARGS=(
  --chain-id "${CHAIN_ID}"
  --output json
);
TX_ARGS=(
  --gas "${GAS}"
  --gas-adjustment "${GAS_ADJUSTMENT}"
  --gas-prices "${GAS_PRICES}"
  -y
  "${QUERY_ARGS[@]}"
);

# create test-user key
if ! ${DOCKER_EXEC} /bin/sh -c "junod keys list" | grep -q test-user; then
  # these are "stable" addresses for testing
  echo "# Creating test-user key ( juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y )";
  ${DOCKER_EXEC} /bin/sh -c "source /opt/test-user.env; echo \$TEST_MNEMONIC | junod keys add test-user --recover" 1>&2;
  echo "# Creating test-user2 key ( juno102fjg5u62qkgsux9z9fl652mw8r98kgcgjv99m )";
  ${DOCKER_EXEC} /bin/sh -c "source /opt/test-user.env; echo \$TEST_MNEMONIC | junod keys add test-user2 --recover --account 2" 1>&2;
fi

# store contract
echo "# Copying contract";
docker cp "${CONTRACT_WASM}" "${CONTAINER_NAME}:/opt/CONTRACT.wasm";
echo -n "# Storing contract ... ";
STORE=$(${DOCKER_EXEC} junod tx wasm store /opt/CONTRACT.wasm -b block --from validator "${TX_ARGS[@]}");
CODE_ID=$(echo "${STORE}" | tail -n +2 | jq -r '.logs[0].events[-1].attributes[0].value');
echo "code_id=${CODE_ID}";

if [ -z "${CODE_ID}" ]; then
  echo "error";
  echo "${STORE}"
  exit 1;
else
  # show gas estimate
  echo "${STORE}" | head -n 1 1>&2;
fi

# instantiate contract
echo -n "# Instantiating contract ... "
INSTANTIATE=$(${DOCKER_EXEC} junod tx wasm instantiate "${CODE_ID}" "${CONTRACT_INSTANTIATE_MESSAGE}" --from validator --label "${CONTRACT_LABEL}" --no-admin "${TX_ARGS[@]}");
# wait for transaction
sleep 10;
CONTRACTS=$(${DOCKER_EXEC} junod query wasm list-contract-by-code "${CODE_ID}" "${QUERY_ARGS[@]}");
CONTRACT_ADDR=$(echo "${CONTRACTS}" | jq -r '.contracts[-1]')
echo "addr=${CONTRACT_ADDR}"
if [ "${CONTRACT_ADDR}" == "null" ]; then
  echo "error" 1>&2;
  echo "${INSTANTIATE}" 1>&2;
  echo "${CONTRACTS}" 1>&2;
  exit 1;
else
  echo "${INSTANTIATE}" | head -n 1 1>&2;
fi

###############################################################################

# output commands to use contract
cat << EOF

export CONTRACT_ADDR="${CONTRACT_ADDR}";

# junod_execute '{MESSAGE}' --from test-user[2]
junod_execute() {
  MESSAGE=\$1;
  shift;
  ${DOCKER_EXEC} junod tx wasm execute "${CONTRACT_ADDR}" "\${MESSAGE}" ${TX_ARGS[@]} "\${@}";
}

# junod_query '{MESSAGE}'
junod_query() {
  MESSAGE=\$1;
  shift;
  ${DOCKER_EXEC} junod query wasm contract-state smart "${CONTRACT_ADDR}" "\${MESSAGE}" ${QUERY_ARGS[@]};
}

junod_destroy() {
  docker stop ${CONTAINER_NAME}
  docker rm ${CONTAINER_NAME}
}

EOF
