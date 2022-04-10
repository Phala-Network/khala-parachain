#!/bin/bash

rm -rf tmp/kernels
mkdir -p tmp/kernels

./target/production/khala-node build-spec --raw --disable-default-bootnode --chain phala-staging > ./tmp/kernels/phala-staging-raw-spec.json
./target/production/khala-node export-genesis-wasm -r --chain phala-staging > ./tmp/kernels/phala-staging-code.wasm
./target/production/khala-node export-genesis-state --chain phala-staging > ./tmp/kernels/phala-staging-state.hex

./target/production/khala-node build-spec --raw --disable-default-bootnode --chain khala-staging > ./tmp/kernels/khala-staging-raw-spec.json
./target/production/khala-node export-genesis-wasm -r --chain khala-staging > ./tmp/kernels/khala-staging-code.wasm
./target/production/khala-node export-genesis-state --chain khala-staging > ./tmp/kernels/khala-staging-state.hex

./target/production/khala-node build-spec --raw --disable-default-bootnode --chain rhala-staging > ./tmp/kernels/rhala-staging-raw-spec.json
./target/production/khala-node export-genesis-wasm -r --chain rhala-staging > ./tmp/kernels/rhala-staging-code.wasm
./target/production/khala-node export-genesis-state --chain rhala-staging > ./tmp/kernels/rhala-staging-state.hex
