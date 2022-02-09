#!/bin/bash

# SKIP_WASM_BUILD=1
cargo run --release --features try-runtime \
  -- try-runtime \
  -l runtime=trace \
  --chain khala-staging-2004 \
  on-runtime-upgrade snap \
  --snapshot-path ./tmp/snapshot.bin |& tee ./tmp/sim.log
