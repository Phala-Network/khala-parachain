import { parse } from "https://deno.land/std/flags/mod.ts";

import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

const parsedArgs = parse(Deno.args, {
  alias: {
    "mnemonic": "m",
    "maxIteration": "max-iter",
    "rpcUrl": "rpc-url",
    "dryRun": "dry-run",
  },
  boolean: [
    "dryRun",
  ],
  string: [
    "mnemonic",
    "rpcUrl",
    "maxIteration",
  ],
  default: {
    rpcUrl: "ws://127.0.0.1:9944",
    dryRun: false,
    maxIteration: 7500,
  },
});

function stringToInteger(n) {
  const parsed = parseInt(n);
  if (isNaN(parsed)) {
    return undefined;
  }

  return parsed;
}

function createSubstrateApi(rpcUrl: string): ApiPromise | null {
  let provider = null;
  if (rpcUrl.startsWith("wss://") || rpcUrl.startsWith("ws://")) {
    provider = new WsProvider(rpcUrl);
  } else if (
    rpcUrl.startsWith("https://") || rpcUrl.startsWith("http://")
  ) {
    provider = new HttpProvider(rpcUrl);
  } else {
    return null;
  }

  return new ApiPromise({
    provider,
    throwOnConnect: true,
    throwOnUnknown: true,
  });
}

await cryptoWaitReady().catch((e) => {
  console.error(e.message);
  Deno.exit(1);
});

const operatorKeyPair = (() => {
  const operatorMnemonic = parsedArgs.mnemonic.toString().trim();
  if (operatorMnemonic === undefined || operatorMnemonic === "") {
    return null;
  }

  try {
    return new Keyring({ type: "sr25519" }).addFromUri(operatorMnemonic, { name: "The migration operator" });
  } catch (e) {
    console.error(`Operator mnemonic invalid: ${e.message}`);
    return null;
  }
})();
if (operatorKeyPair !== null) {
  console.log(`Operator: ${operatorKeyPair.address}`);
}

const maxIteration = stringToInteger(parsedArgs.maxIteration) ?? 100;

const api = createSubstrateApi(parsedArgs.rpcUrl);
if (api === null) {
  console.error(`Invalid RPC URL "${parsedArgs.rpcUrl}"`);
  Deno.exit(1);
}

api.on("error", (e) => {
  console.error(`Polkadot.js error: ${e.message}"`);
  Deno.exit(1);
});

await api.isReady.catch((e) => console.error(e));

let sentTxAt: undefined | Number = undefined;
await api.rpc.chain.subscribeFinalizedHeads(async (finalizedHeader) => {
  const finalizedBlockHash = finalizedHeader.hash.toHex();
  const finalizedBlockNumber = finalizedHeader.number.toNumber();

  const latestHeader = await api.rpc.chain.getHeader();
  const latestBlockHash = latestHeader.hash.toHex();
  const latestBlockNumber = latestHeader.number.toNumber();

  console.info(
    `best: #${latestBlockNumber} (${latestBlockHash}), finalized #${finalizedBlockNumber} (${finalizedBlockHash})`
  );

  if (sentTxAt !== undefined && sentTxAt >= finalizedBlockNumber) {
    console.debug("Waiting extrinsic finalize...");

    return;
  }

  console.info(`Sending "phalaBasePool.removeUnusedLock(maxIterations)`);
  const txPromise = api.tx.phalaBasePool.removeUnusedLock(maxIteration);
  console.info(`Call hash: ${txPromise.toHex()}`);
  const txHash = await txPromise.signAndSend(operatorKeyPair, { nonce: -1 });
  console.info(`Transaction hash: ${txHash.toHex()}`);

  sentTxAt = latestBlockNumber;
});
