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

> You can get latest Polkadot Linux release from https://github.com/paritytech/polkadot/releases
> 
> For macOS, you have to build from source

```bash
cp <path-to-polkadot> ./polkadot-launch/bin/
cp ./target/release/khala ./polkadot-launch/bin/
```

3. Start a local test net

```bash
cd polkadot-launch
yarn
yarn start ./thala_dev.config.json
```

4. (Optional) Inject session key.

> Only required for `*_local.config.json`

Please make sure to use the correct session key json file.

```bash
cd scripts/js
yarn
node insert_session_key.js
```
