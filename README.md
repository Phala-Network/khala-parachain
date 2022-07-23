# Khala Parachain

Khala parachain repo.

## Build Note

### For production build

`cargo build --profile production`

### For testnet build

`cargo build --profile testnet`

### Special note for macOS

You have to install LLVM from Homebrew because Xcode bundled doesn't have WASM target

```
brew install llvm
```

As formula note shows binaries locate at `/opt/homebrew/opt/llvm/bin`
(You can revisit it via `brew info llvm`, beware Intel Mac has different location)

Make alias for `ar` and `ranlib` to ensure use custom installed LLVM instead of Xcode bundled

```
cd /opt/homebrew/opt/llvm/bin
ln -s llvm-ar ar
ln -s llvm-ar ranlib
```

*NOTE: You have to re-do this step after you upgrade or reinstall LLVM*

Before compiling `khala-node`, add custom installed LLVM to `$PATH`

```
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
```

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
KEY_FILE=session_key.json.example node insert_session_key.js
```
