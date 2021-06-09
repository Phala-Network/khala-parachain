#!/bin/bash

mkdir -p tmp/kernels

./target/release/khala-node export-genesis-wasm -r --chain khala-staging > ./tmp/kernels/staging-code.wasm
./target/release/khala-node export-genesis-state --chain khala-staging > ./tmp/kernels/staging-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-local-2004 > ./tmp/kernels/local-code.wasm
./target/release/khala-node export-genesis-state --chain khala-local-2004 > ./tmp/kernels/local-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-local-2000 > ./tmp/kernels/local-code-2000.wasm
./target/release/khala-node export-genesis-state --chain khala-local-2000 > ./tmp/kernels/local-state-2000.hex

./target/release/khala-node export-genesis-wasm -r --chain whala > ./tmp/kernels/whala-code.wasm
./target/release/khala-node export-genesis-state --chain whala > ./tmp/kernels/whala-state.hex
