// USAGE:
//   ENDPOINT=ws://127.0.0.1:9944 DRY_RUN=0 node insertSessionKey.js

require('dotenv').config();

const { ApiPromise, Keyring, WsProvider } = require('@polkadot/api');
const { u8aToHex } = require('@polkadot/util');

const WS_ENDPOINT = process.env.ENDPOINT || 'ws://localhost:9944';
const SESSION_KEY = process.env.SESSION_KEY || '//Alice//session';
const DRY_RUN = process.env.DRY_RUN == '1' || false;

async function main () {
    const wsProvider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({ provider: wsProvider, types: undefined });

    const keyringSr = new Keyring({ type: 'sr25519' });
    const aura = keyringSr.addFromUri(SESSION_KEY);

    // Session Keys:
    //   aura: AuraId,
    const keys = api.createType('Keys', [
        aura.publicKey
    ]);
    const opaqueSessionKey = keys.toHex();

    let rpcResult = {};
    if (!DRY_RUN) {
        const pubkeyAura = u8aToHex(aura.publicKey);
        await api.rpc.author.insertKey('aura', SESSION_KEY, pubkeyAura);
        const inserted = (await api.rpc.author.hasSessionKeys(opaqueSessionKey)).toJSON();
        rpcResult = {
            pubkeyAura,
            inserted,
        };
    }

    console.log({
        opaqueSessionKey,
        rpcResult,
    });
}

main().catch(console.error).finally(() => process.exit());
