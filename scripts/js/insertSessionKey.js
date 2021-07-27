// USAGE:
//   ENDPOINT=ws://127.0.0.1:9944 DRY_RUN=0 node insertSessionKey.js

require('dotenv').config();

const fs = require('fs');
const { ApiPromise, Keyring, WsProvider } = require('@polkadot/api');
const { u8aToHex } = require('@polkadot/util');
const typedefs = require('@phala/typedefs').khalaDev;

const KEY_FILE = process.env.KEY_FILE;
const WS_ENDPOINT = process.env.ENDPOINT || 'ws://localhost:9944';
const SESSION_KEY = process.env.SESSION_KEY || '//Alice//session';
const DRY_RUN = process.env.DRY_RUN == '1' || false;

async function main () {
    // load ops
    let operations;
    if (KEY_FILE) {
        const file = fs.readFileSync(KEY_FILE, { encoding: 'utf-8' });
        operations = JSON.parse(file);
    } else {
        operations = [{
            endpoint: WS_ENDPOINT,
            key: SESSION_KEY,
        }];
    }

    const keyringSr = new Keyring({ type: 'sr25519' });

    for (const {endpoint, key} of operations) {
        const wsProvider = new WsProvider(endpoint);
        const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });
        const aura = keyringSr.addFromUri(key);
    
        // Session Keys:
        //   aura: AuraId,
        const keys = api.createType('Keys', [
            aura.publicKey
        ]);
        const opaqueSessionKey = keys.toHex();
    
        let rpcResult = {};
        if (!DRY_RUN) {
            const pubkeyAura = u8aToHex(aura.publicKey);
            await api.rpc.author.insertKey('aura', key, pubkeyAura);
            const inserted = (await api.rpc.author.hasSessionKeys(opaqueSessionKey)).toJSON();
            rpcResult = {
                pubkeyAura,
                inserted,
            };
        }
    
        console.log({
            endpoint,
            opaqueSessionKey,
            rpcResult,
        });
    }
}

main().catch(console.error).finally(() => process.exit());
