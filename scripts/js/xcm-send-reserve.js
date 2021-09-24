// Example:
//   SOURCE=2004 DEST=2005 TO=43L9xU7zKU2ByNKvsgb4QLnhBU4pVvfbphh52TeH8byRtPuj AMOUNT=9 node xcm-send-reserve.js
require('dotenv').config();

const { ApiPromise, Keyring, WsProvider } = require('@polkadot/api');
const BN = require('bn.js');
const typedefs = require('@phala/typedefs').phalaDev;
const bn1e12 = new BN(10).pow(new BN(12));
const { decodeAddress } = require('@polkadot/util-crypto');

async function main () {

    const wsProvider = new WsProvider(process.env.ENDPOINT || "ws://127.0.0.1:9944");
    const api = await ApiPromise.create({ provider: wsProvider, types: typedefs });

    const keyring = new Keyring({ type: 'sr25519' });
    const sender = keyring.addFromUri('//Alice');
    const source = Number(process.env.SOURCE);
    const dest = Number(process.env.DEST);
    const recipient = process.env.TO;
    const amount = bn1e12.muln(Number(process.env.AMOUNT));

    const unsub = await api.tx.polkadotXcm.reserveTransferAssets(
        api.createType('VersionedMultiLocation', {
            V1: api.createType('MultiLocationV1', {
                parents: api.createType('Compact<U8>', 1),
                interior: api.createType('JunctionsV1', {
                    X1: api.createType('JunctionV1', {
                        Parachain: api.createType('Compact<U32>', dest)
                    })
                })
            })
        }),
        api.createType('VersionedMultiLocation', {
            V1: api.createType('MultiLocationV1', {
                parents: api.createType('Compact<U8>', 1),
                interior: api.createType('JunctionsV1', {
                    X1: api.createType('JunctionV1', {
                        AccountId32: {
                            network: api.createType('NetworkId', 'Any'),
                            id: api.createType('AccountId', recipient)
                        }
                    })
                })
            })
        }),
        api.createType('VersionedMultiAssets', {
            V1: api.createType('MultiAssetsV1', [
                api.createType('MultiAssetV1', {
                    id: api.createType('XcmAssetId', {
                        concrete: api.createType('MultiLocation', {
                            parents: api.createType('Compact<U8>', 1),
                            interior: api.createType('JunctionsV1', {
                                X1: api.createType('JunctionV1', {
                                    Parachain: api.createType('Compact<U32>', source)
                                })
                            })
                        })
                    }),
                    fungibility: api.createType('Fungibility', {
                        fungible: api.createType('Compact<U128>', amount)
                    })
                })
            ])
        }),
        0,
        api.createType('Compact<U64>', new BN('6000000000'))
    )
    .signAndSend(sender, (result) => {
        console.log(`Current status is ${result.status}`);
        if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
        } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            unsub();
        }
    });
}

main().catch(console.error).finally(() => process.exit());
