import { parse } from "https://deno.land/std/flags/mod.ts";

import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

import { gql, request } from "https://deno.land/x/graphql_request/mod.ts";

const query = gql`
{
  basePoolsConnection(orderBy: id_ASC, first: 10000, where: {kind_eq: StakePool, withdrawingShares_not_eq: "0"}) {
    edges {
      node {
        id
      }
    }
  }
}
`;

const parsedArgs = parse(Deno.args, {
  alias: {
    "mnemonic": "m",
    "rpcUrl": "rpc-url",
    "gqlUrl": "gql-url",
    "dryRun": "dry-run",
  },
  boolean: [
    "dryRun",
  ],
  string: [
    "mnemonic",
    "rpcUrl",
    "gqlUrl",
  ],
  default: {
    rpcUrl: "ws://127.0.0.1:9944",
    gqlUrl: "https://squid.subsquid.io/phala-computation/graphql",
    dryRun: false,
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

const data = await request(parsedArgs.gqlUrl, query);
const pids = data?.basePoolsConnection?.edges?.map((i: any) => stringToInteger(i?.node?.id));

console.log(pids);

// // the weight too large that can't batch, so we comment here
// const txPromise = api.tx.utility.forceBatch(
//   pids.map(
//     (pid: any) => {
//       return [
//         api.tx.phalaStakePoolv2.checkAndMaybeForceWithdraw(pid),
//       ]
//     }
//   ).flat()
// );

// console.info(`Sending phalaStakePoolv2.checkAndMaybeForceWithdraw(pid)`);
// console.info(`Call hash: ${txPromise.toHex()}`);
// const txHash = await txPromise.signAndSend(operatorKeyPair, { nonce: -1 });
// console.info(`Transaction hash: ${txHash.toHex()}`);

for (const pid of pids) {
  console.info(`Sending phalaStakePoolv2.checkAndMaybeForceWithdraw(${pid})`);
  const txPromise = api.tx.phalaStakePoolv2.checkAndMaybeForceWithdraw(pid);
  console.info(`Call hash: ${txPromise.toHex()}`);
  const txHash = await txPromise.signAndSend(operatorKeyPair, { nonce: -1 });
  console.info(`Transaction hash: ${txHash.toHex()}`);
}

Deno.exit(0);
