#!/bin/bash

REV=$(git rev-parse HEAD)

rm -rf "./tmp/wasm/$REV"
mkdir -p "./tmp/wasm/$REV/release"
mkdir -p "./tmp/wasm/$REV/production"

cp ./target/release/wbuild/*-runtime/*.compressed.wasm "./tmp/wasm/$REV/release/"
cp ./target/production/wbuild/*-runtime/*.compressed.wasm "./tmp/wasm/$REV/production/"
