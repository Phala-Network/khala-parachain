#!/bin/bash

rm -rf tmp/kernels
mkdir -p tmp/kernels

./target/release/khala-node export-genesis-wasm -r --chain phala-staging > ./tmp/kernels/phala-staging-code.wasm
./target/release/khala-node export-genesis-state --chain phala-staging > ./tmp/kernels/phala-staging-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-staging > ./tmp/kernels/khala-staging-code.wasm
./target/release/khala-node export-genesis-state --chain khala-staging > ./tmp/kernels/khala-staging-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-local-2004 > ./tmp/kernels/local-code.wasm
./target/release/khala-node export-genesis-state --chain khala-local-2004 > ./tmp/kernels/local-state.hex

./target/release/khala-node export-genesis-wasm -r --chain khala-local-2000 > ./tmp/kernels/local-code-2000.wasm
./target/release/khala-node export-genesis-state --chain khala-local-2000 > ./tmp/kernels/local-state-2000.hex

./target/release/khala-node export-genesis-wasm -r --chain rhala-staging > ./tmp/kernels/rhala-staging-code.wasm
./target/release/khala-node export-genesis-state --chain rhala-staging > ./tmp/kernels/rhala-staging-state.hex
