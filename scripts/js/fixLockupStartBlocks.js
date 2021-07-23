require('dotenv').config();

const fs = require('fs');
const { ApiPromise, Keyring, WsProvider } = require('@polkadot/api');

const IN_FILE = process.env.IN_FILE || './tmp/locks1.json';
const WS_ENDPOINT = process.env.ENDPOINT || 'wss://khala.phala.network/ws';
const DRY_RUN = process.env.DRY_RUN == '1' || false;

async function main () {
    // input data
    const jsonContent = fs.readFileSync(IN_FILE, { encoding: 'utf-8' });
    const data = JSON.parse(jsonContent);
    const targets = data.filter(r => r.type == 'vesting');

    // rpc
    const wsProvider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({ provider: wsProvider, types: undefined });
    const h = await api.rpc.chain.getBlockHash();

    // get all existing data
    const addrs = targets.map(r => r.target);
    const origDataOpt = await api.query.vesting.vesting.multi(addrs);
    const origData = origDataOpt.map(r => r.unwrap());

    console.log('==== Original Data Inspection ====');
    console.log(origData.map(d => d.toJSON()));
    console.log(`${origData.length} records in total`);
    console.log('Unwrap successfully?', origData.every(x => !!x));
    console.log('All starting block == 0?', origData.every(d => d.startingBlock.toNumber() == 0))

    // fix the data
    const fixedData = origData.map(r => {
        const newRecord = api.createType('VestingInfo', {
            locked: r.locked,
            perBlock: r.perBlock,
            startingBlock: 50000,
        });
        return newRecord;
    });
    console.log(fixedData.map(r => r.toJSON()));
    console.log(
        'Data length not changed?',
        fixedData
            .map((r, i) => r.toU8a().length == origData[i].toU8a().length)
            .every(x => x)
    );
    console.log(
        'Fields are changed properly?',
        fixedData
            .map((r, i) =>
                r.locked.eq(origData[i].locked)
                && r.perBlock.eq(origData[i].perBlock)
                && !r.startingBlock.eq(origData[i].startingBlock)
            )
            .every(x => x)
    )

    // final check of the raw storage
    const keys = addrs.map(a => api.query.vesting.vesting.key(a));
    const rawValues = await Promise.all(keys.map(k => api.rpc.state.getStorage(k, h)));
    console.log(
        'Raw storage values are correct?',
        origData
            .map((r, i) => r.toHex() == rawValues[i].toHex())
            .every(x => x)
    );

    // sample 5 values
    console.log('==== Raw Data Samples ====');
    console.log(
        fixedData
            .slice(0, 5)
            .map((r, i) => `[${i}] ORIG: ${origData[i].toHex()}\n     NEW: ${r.toHex()}`)
            .join('\n')
    );


    // Prepare for tx
    console.log('==== Prepare System::set_storage Call ====')
    const setStoragePayload = api.createType(
        'Vec<KeyValue>',
        keys.map((k, i) =>
            api.createType('KeyValue', [k, fixedData[i].toHex()]),
        )
    );
    const innerCall = api.tx.system.setStorage(setStoragePayload);
    console.log(innerCall.toJSON());

    if (DRY_RUN) {
        console.warn('Dryrun. Exiting...');
        return;
    }

    const keyring = new Keyring({ type: 'sr25519' });
    const root = keyring.addFromUri(process.env.PRIVKEY);
    const r = await api.tx.sudo.sudo(innerCall).signAndSend(root);
    console.log(r.toHuman());
}

main().catch(console.error).finally(() => process.exit());
