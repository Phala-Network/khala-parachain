#!/bin/bash

steps=20
repeat=10
khalaOutput=./runtime/khala/src/weights/
khalaChain=khala-dev
pallets=(
	pallet_balances
	pallet_multisig
	pallet_proxy
	pallet_timestamp
	pallet_utility
)

for p in ${pallets[@]}
do
	./target/release/khala-collator benchmark \
		--chain $khalaChain \
		--execution wasm \
		--wasm-execution compiled \
		--pallet $p  \
		--extrinsic '*' \
		--steps $steps  \
		--repeat $repeat \
		--raw  \
		--output $khalaOutput
done
