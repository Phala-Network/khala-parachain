#!/bin/bash

# Check if gh is available
if ! command -v gh &> /dev/null
then
    echo "gh could not be found. Please install gh (https://github.com/cli/cli) and try again."
    exit 1
fi

# Get the current tag if available
CURRENT_TAG=$(git describe --exact-match --tags 2> /dev/null)

# If there is no current tag, raise an error
if [ -z "${CURRENT_TAG}" ]; then
    echo "Error: The git tree is not on any tag. Please checkout a commit with a tag and try again."
    exit 1
else
    echo "Current tag: ${CURRENT_TAG}"
fi

# cargo build --profile production

"./scripts/export_runtime.sh" "$CURRENT_TAG"
WASM_BASE="./tmp/wasm/$CURRENT_TAG/production"

gh release create "${CURRENT_TAG}" \
    -t "Phala & Khala Network Runtime ${CURRENT_TAG}" \
    -d --generate-notes \
    "$WASM_BASE"/*_parachain_runtime.compact.compressed.wasm
    #-t "${CURRENT_TAG}" -n "Release ${CURRENT_TAG}" \

echo "Release ${CURRENT_TAG} created and uploaded."

