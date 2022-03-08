# Khala Parachain

Khala parachain repo.

## Build Note

### For production build

`cargo build --profile production`

### For testnet build

`cargo build --profile testnet`

## Development Note

### Run a local testnet cluster

1. Build

    ```bash
    cargo build --release
    ```

2. Prepare binaries

    ```bash
    cp <path-to-polkadot> ./polkadot-launch/bin/
    cp ./target/release/khala ./polkadot-launch/bin/
    ```

3. Start a local cluster

    ```bash
    cd polkadot-launch
    yarn
    yarn start ./khala.config.json
    ```

4. Inject session key if necessary. Please make sure to use the correct session key json file.

    ```bash
    cd scripts/js
    yarn
    node insert_session_key.js
    ```
