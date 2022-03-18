#! /bin/bash

# work from script directory
cd "$(dirname $0)";

CONTAINER="juno_node_1";
INSTANTIATE_MESSAGE='{}';
LABEL="cosmwasm-chess";
WASM="../target/wasm32-unknown-unknown/release/cosmwasm_chess.wasm";


###############################################################################

if [[ ! -f "${WASM}" ]]; then
  echo "WASM contract not found";
  echo "Run 'cargo wasm' to build";
  exit 1;
fi

# make sure jq is installed
if [[ ! $(command -v jq) ]]; then
  echo "jq not found";
  echo "On a mac, try 'brew install jq'"
fi

# make sure local node is running
if [[ ! $(docker ps | grep "${CONTAINER}") ]]; then
  echo "Local node not found";
  echo "From root of https://github.com/CosmosContracts/juno";
  echo "Run 'docker-compose up'";
  exit 1;
fi


EXEC="docker exec -it ${CONTAINER}";
QUERY_ARGS=(
  --chain-id testing
  --output json
);
TX_ARGS=(
  --from test-user
  --gas auto
  --gas-adjustment 1.3
  --gas-prices 0.01ucosm
  -y
  "${QUERY_ARGS[@]}"
);


# create test-user key
if [[ ! $(${EXEC} /bin/sh -c "junod keys list" | grep test-user) ]]; then
  echo "Creating test-user key";
  ${EXEC} /bin/sh -c "source /opt/test-user.env; echo \$TEST_MNEMONIC | junod keys add test-user --recover";
fi


# store contract
echo "Copying contract";
docker cp "${WASM}" "${CONTAINER}:/opt/CONTRACT.wasm";
echo -n "Storing contract ... ";
STORE=$(${EXEC} junod tx wasm store /opt/CONTRACT.wasm -b block "${TX_ARGS[@]}");
CODE_ID=$(echo "${STORE}" | tail -n +2 | jq -r '.logs[0].events[-1].attributes[0].value');
if [ -z "${CODE_ID}" ]; then
  echo "error";
  echo "${STORE}";
  exit 1;
fi
echo "code_id=${CODE_ID}"


# instantiate contract
echo -n "Instantiating contract ... "
INSTANTIATE=$(${EXEC} junod tx wasm instantiate "${CODE_ID}" "${INSTANTIATE_MESSAGE}" --label "${LABEL}" --no-admin "${TX_ARGS[@]}")
# wait for transaction
sleep 5;
CONTRACTS=$(${EXEC} junod query wasm list-contract-by-code "${CODE_ID}" "${QUERY_ARGS[@]}");
CONTRACT_ADDR=$(echo "${CONTRACTS}" | jq -r '.contracts[-1]')
if [ "${CONTRACT_ADDR}" == "null" ]; then
  echo "error";
  echo "${INSTANTIATE}";
  echo "${CONTRACTS}";
  exit 1;
fi
echo "addr=${CONTRACT_ADDR}"


# output commands to use contract
echo
echo "To execute contract, replace '{MESSAGE}' in this command:"
echo ${EXEC} junod tx wasm execute "${CONTRACT_ADDR}" '{MESSAGE}' "${TX_ARGS[@]}"
echo
echo "To query contract, replace '{MESSAGE}' in this command:"
echo ${EXEC} junod query wasm contract-state smart "${CONTRACT_ADDR}" '{MESSAGE}' "${QUERY_ARGS[@]}"
echo