import { parse } from "https://deno.land/std/flags/mod.ts";

import { ApiPromise, HttpProvider, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

const parsedArgs = parse(Deno.args, {
    alias: {
        "rpcUrl": "rpc-url",
    },
    string: [
        "rpcUrl",
        "out",
    ],
    default: {
        rpcUrl: "ws://127.0.0.1:9944",
        out: "dumped.json"
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

const finalizedHash = await api.rpc.chain.getFinalizedHead()
const finalizedHeader = await api.rpc.chain.getHeader(finalizedHash)
const finalizedNumber = finalizedHeader.number.toNumber()

const apiAtFinalized = await api.at(finalizedHash);

const [workers, workersPools, pools] = await Promise.all([
    apiAtFinalized.query.phalaRegistry.workers.entries(),
    apiAtFinalized.query.phalaStakePoolv2.subAccountPreimages.entries(),
    apiAtFinalized.query.phalaBasePool.pools.entries(),
]);

const json = {
    blockNumber: finalizedNumber,
    workers: workers.map(([_key, worker]) => worker.toJSON()),
    workersPools: workersPools.reduce<Record<string, unknown>>(
        (acc, [_miner, poolIdAndWorker]) => {
            // acc[miner.args[0].toString()] = pid_and_worker.toJSON()

            const poolId = (poolIdAndWorker.toJSON() as any[])[0]
            const worker = (poolIdAndWorker.toJSON() as any[])[1]
            acc[worker] = poolId

            return acc
        },
        {}
    ),
    pools: pools.map(([_key, pool]) => pool.toJSON()?.stakePool?.basepool).filter(i => i),
}

Deno.writeTextFileSync(parsedArgs.out, JSON.stringify(json, null, 2))

Deno.exit(0)
