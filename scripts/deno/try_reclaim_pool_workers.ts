import { parse } from "https://deno.land/std/flags/mod.ts";

import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

import { gql, request } from "https://deno.land/x/graphql_request/mod.ts";

const query = gql`
{
  sessionsConnection(orderBy: id_ASC, first: 10000, where: {state_eq: WorkerCoolingDown}) {
    edges {
      node {
        worker {
          id
        }
        stakePool {
          basePool {
            pid
          }
        }
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

Array.prototype.eachSlice = function (size: number, callback: (slicedArray: any[]) => void) {
  let i = 0, l = this.length;
  for (; i < l; i += size){
    callback.call(this, this.slice(i, i + size))
  }
};

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
const poolWorkers = data?.sessionsConnection?.edges?.map((i: any) => {
  return {
    worker: i?.node?.worker?.id,
    pid: stringToInteger(i?.node?.stakePool?.basePool?.pid),
  }
});

// console.log(poolWorkers);

let txPromises: Promise<any>[] = [];
poolWorkers.eachSlice(50, async (slicedPoolWorkers: any[]) => {
  txPromises.push(
    api.tx.utility.forceBatch(
      slicedPoolWorkers.map(
        (slicedPoolWorker: any) => {
          const pid = slicedPoolWorker.pid;
          const worker = slicedPoolWorker.worker;
          return [
            api.tx.phalaStakePoolv2.reclaimPoolWorker(pid, worker),
          ]
        }
      ).flat()
    )
  );
})

for (const txPromise of txPromises) {
  console.info(`Sending phalaStakePoolv2.reclaimPoolWorker(pid, worker) in batch`);
  console.info(`Call hash: ${txPromise.toHex()}`);
  const txHash = await txPromise.signAndSend(operatorKeyPair, { nonce: -1 });
  console.info(`Transaction hash: ${txHash.toHex()}`);
}

Deno.exit(0);
