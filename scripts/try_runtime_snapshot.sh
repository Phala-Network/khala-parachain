#!/bin/bash

# Useful when doing intensive testing in a short period. Can dump a snapshot by `try_runtime.sh`
# and load it here.

# SKIP_WASM_BUILD=1
cargo run --release --features try-runtime \
  -- try-runtime \
  -l runtime=trace \
  --no-spec-name-check \
  --chain khala-staging-2004 \
  on-runtime-upgrade snap \
  --snapshot-path ./tmp/snapshot.bin |& tee ./tmp/sim.log
