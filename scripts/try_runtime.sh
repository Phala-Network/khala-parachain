#!/bin/bash

# Trigger `try-runtime` as a specific block (and save the snapshot to the tmp dir)

# SKIP_WASM_BUILD=1
cargo run --release --features try-runtime \
  -- try-runtime \
  -d /tmp/blackhole \
  --no-spec-name-check \
  --chain khala-staging-2004 \
  on-runtime-upgrade live \
  -u ws://127.0.0.1:9944 \
  --at 0x1c3c6c0ef3f1a8fcb53608c09a8f0a25367fca6381ea73d3ecc8ad5783040b04 \
  --snapshot-path ./tmp/snapshot.bin |& tee ./tmp/sim.log



# potential useful logger filtering expression
# -l trace,soketto=warn,jsonrpsee_ws_client=warn,remote-ext=warn,trie=warn,wasmtime_cranelift=warn \
