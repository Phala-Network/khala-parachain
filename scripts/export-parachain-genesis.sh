#!/bin/bash

mkdir -p tmp/kernels

./target/release/khala-node export-genesis-wasm -r --chain khala-staging > ./tmp/kernels/staging-genesis.wasm
./target/release/khala-node export-genesis-state --chain khala-staging > ./tmp/kernels/staging-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-local > ./tmp/kernels/local-genesis.wasm
./target/release/khala-node export-genesis-state --chain khala-local > ./tmp/kernels/local-state.hex
