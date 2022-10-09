#!/bin/bash

# Rhala
# ./target/release/khala-node try-runtime --chain rhala-staging-2004 on-runtime-upgrade live -u "wss://rhala-api.phala.network:443/ws" --at 0xf83a6f466ac30169274e90ad3989f9e95252f24c1c19f0a3f63476430902c333

# Khala
./target/release/khala-node try-runtime --chain khala-staging-2004 --execution native on-runtime-upgrade live -u "wss://khala.api.onfinality.io:443/public-ws" --at 0xa871b02f6cc58af0abfe9bc74173fefeaa42f1042375f499c37d0bfa51508c2f