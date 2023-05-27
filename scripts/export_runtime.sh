#!/bin/bash

REV=$(git rev-parse HEAD)
NAME="${1:-$REV}"

rm -rf "./tmp/wasm/$NAME"
mkdir -p "./tmp/wasm/$NAME/release"
mkdir -p "./tmp/wasm/$NAME/production"

cp ./target/production/wbuild/*-runtime/*.compressed.wasm "./tmp/wasm/$NAME/production/"
