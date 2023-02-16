import { parse } from "https://deno.land/std/flags/mod.ts";

import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

const parsedArgs = parse(Deno.args, {
  alias: {
    "rpcUrl": "rpc-url",
    "dryRun": "dry-run",
  },
  boolean: [
    "dryRun",
  ],
  string: [
    "rpcUrl",
  ],
  default: {
    rpcUrl: "ws://127.0.0.1:9944",
    dryRun: false,
  },
});

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

let pending: any = (await api.rpc.author.pendingExtrinsics()).toJSON();
console.log(
  'before',
  JSON.stringify(pending, undefined, 2)
)

const toRemove = pending.map((tx: any) => ({'Extrinsic': tx}))
const r = await api.rpc.author.removeExtrinsic(toRemove);
console.log('Removing', r);

pending = (await api.rpc.author.pendingExtrinsics()).toJSON();
console.log(
  'after',
  JSON.stringify(pending, undefined, 2)
)

console.log("Job finished")
Deno.exit(0)
