#!/bin/bash

# SKIP_WASM_BUILD=1
cargo run --release --features try-runtime \
  -- try-runtime \
  -d /tmp/blackhole \
  --chain khala-staging-2004 \
  on-runtime-upgrade live \
  -u ws://127.0.0.1:9944 \
  --at 0xf49f9fbde81806d37cd9033c18da122d45e623c44523e3a5124dccc52b4a2d5a \
  --snapshot-path ./tmp/snapshot.bin |& tee ./tmp/sim.log

# -l trace,soketto=warn,jsonrpsee_ws_client=warn,remote-ext=warn,trie=warn,wasmtime_cranelift=warn \
