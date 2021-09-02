// USAGE:
//   ENDPOINT=ws://127.0.0.1:9944 DRY_RUN=0 node insertSessionKey.js

require('dotenv').config();

const { ApiPromise, WsProvider } = require('@polkadot/api');
const WS_ENDPOINT = process.env.ENDPOINT || 'ws://localhost:19944';

const FRNK = '0x46524e4b';

function hasJustification (api, blockData) {
    let justification = blockData.justifications.toJSON()
    if (justification) {
      justification = api.createType(
        'JustificationToSync',
        justification.reduce(
          (acc, current) => (current[0] === FRNK ? current[1] : acc),
          '0x'
        )
      )
    }
    const hasJustification = justification
      ? justification.toHex().length > 2
      : false;
    return hasJustification;
}

async function main () {
    const wsProvider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({
        provider: wsProvider, types: {
            'JustificationToSync': 'Option<EncodedJustification>',
        }
    });

    const h0 = await api.rpc.chain.getFinalizedHead();
    const b0 = await api.rpc.chain.getBlock(h0);
    const latest = b0.block.header.number.toNumber();

    for (let i = latest-100; i <= latest; i++) {
      const h = await api.rpc.chain.getBlockHash(i);
      const b = await api.rpc.chain.getBlock(h);
      console.log(`${b.block.header.number.toNumber()} ${hasJustification(api, b)}`);
    }
}

main().catch(console.error).finally(() => process.exit());
