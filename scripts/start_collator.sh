#!/bin/bash

NODE_NAME=${NODE_NAME:-"khala-collator"}
P2P_PORT=${P2P_PORT:-"30333"}
WS_PORT=${WS_PORT:-"9944"}
RPC_PORT=${WS_PORT:-"9933"}
BIN_PATH=${BIN_PATH:-"$(dirname $(dirname $(readlink -f "$0")))/target/release"}
DATA_PATH=${DATA_PATH:-"$HOME/data/$NODE_NAME"}
CHAIN_NAME=${CHAIN_NAME:-"khala"}

"$BIN_PATH"/khala-node \
  --chain "$CHAIN_NAME" \
  --base-path "$DATA_PATH" \
  --name "$NODE_NAME" \
  --collator \
  --port "$P2P_PORT" \
  --rpc-port "$RPC_PORT" \
  --ws-port "$WS_PORT" \
  --ws-max-connections 200 \
  --rpc-cors all
