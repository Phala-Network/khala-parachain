// USAGE:
//   ENDPOINT=ws://127.0.0.1:9944 DRY_RUN=0 node insertSessionKey.js

require('dotenv').config();

const fs = require('fs');
const { ApiPromise, Keyring, WsProvider } = require('@polkadot/api');
const { u8aToHex, stringToHex } = require('@polkadot/util');
const typedefs = require('@phala/typedefs').khalaDev;

const KEY_FILE = process.env.KEY_FILE;
const WS_ENDPOINT = process.env.ENDPOINT || 'ws://localhost:9944';
const SESSION_KEY = process.env.SESSION_KEY || '//Alice//session';
const DRY_RUN = process.env.DRY_RUN == '1' || false;

async function insertKey(api, suri, keyringType, keyType) {
    const keyring = keyringType.addFromUri(suri);

    if (DRY_RUN) {
        return;
    }

    const pubkey = u8aToHex(keyring.publicKey);
    await api.rpc.author.insertKey(keyType, suri, pubkey);
    const inserted = (await api.rpc.author.hasKey(pubkey, keyType)).toJSON();

    if (inserted) {
        console.log(`Set "${keyType}" successful, public key "${pubkey}"`)
    } else {
        console.log(`Set "${keyType}" failed, public key "${pubkey}"`)
        return;
    }

    const encodedLeyType = stringToHex(keyType.split('').reverse().join(''));
    const owner = await api.query.session.keyOwner([encodedLeyType, pubkey]);
    if (!owner.isSome) {
        console.warn(`Session key not found on-chain: ${keyType}-${pubkey}`);
    }

    return inserted;
}

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

    if (DRY_RUN) {
        console.log("Dry run mode, will not actually inject keys.");
    }

    for (const {endpoint, key} of operations) {
        const wsProvider = new WsProvider(endpoint);
        const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

        console.log(`Connected to "${endpoint}"`);
        await insertKey(api, key, keyringSr, "aura");
    }
}

main().catch(console.error).finally(() => process.exit());
