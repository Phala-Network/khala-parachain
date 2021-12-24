require('dotenv').config();

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const BN = require('bn.js');
const ethers = require('ethers');
const BridgeJson = require('./Bridge.json');

const bn1e12 = new BN(10).pow(new BN(12));
const bn1e18 = new BN(10).pow(new BN(18));
const khalaParaId = 2004;
const karuraParaId = 2000;
const bridgeAddressOnRinkeby = '0x0712Cf53B9fA1A33018d180a4AbcC7f1803F55f4';

async function transferPHAFromKhalaToKarura(khalaApi, sender, recipient, amount) {
    console.log(`Transfer PHA from Khala to Karura...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.xcmTransfer.transferNative(
            khalaApi.createType('XcmV1MultiLocation', {
                parents: 1,
                interior: khalaApi.createType('Junctions', {
                    X2: [
                        khalaApi.createType('XcmV1Junction', {
                            Parachain: khalaApi.createType('Compact<U32>', karuraParaId)
                        }),
                        khalaApi.createType('XcmV1Junction', {
                            AccountId32: {
                                network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                                id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                            }
                        }),
                    ]
                })
            }),
            amount,
            6000000000
        ).signAndSend(sender, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
    });
}

async function transferPHAFromKaruraToKhala(karuraApi, sender, recipient, amount) {
    console.log(`Transfer PHA from Karura to Khala...`);
    return new Promise(async (resolve) => {
        const unsub = await karuraApi.tx.xTokens.transfer(
            karuraApi.createType('AcalaPrimitivesCurrencyCurrencyId', {
                // 170 is PHA registered in kurura runtime
                Token: karuraApi.createType('AcalaPrimitivesCurrencyTokenSymbol', 170)
            }),
            amount,
            karuraApi.createType('XcmVersionedMultiLocation', {
                V1: karuraApi.createType('XcmV1MultiLocation', {
                    parents: 1,
                    interior: karuraApi.createType('Junctions', {
                        X2: [
                            karuraApi.createType('XcmV1Junction', {
                                Parachain: karuraApi.createType('Compact<U32>', khalaParaId)
                            }),
                            karuraApi.createType('XcmV1Junction', {
                                AccountId32: {
                                    network: karuraApi.createType('XcmV0JunctionNetworkId', 'Any'),
                                    id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                                }
                            }),
                        ]
                    })
                })
            }),
            6000000000
        ).signAndSend(sender, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
    });
}

async function transferKARFromKaruraToKhala(karuraApi, sender, recipient, amount) {
    console.log(`Transfer KAR from Karura to Khala...`);
    return new Promise(async (resolve) => {
        const unsub = await karuraApi.tx.xTokens.transfer(
            karuraApi.createType('AcalaPrimitivesCurrencyCurrencyId', {
                // 128 is KAR in kurura runtime
                Token: karuraApi.createType('AcalaPrimitivesCurrencyTokenSymbol', 128)
            }),
            amount,
            karuraApi.createType('XcmVersionedMultiLocation', {
                V1: karuraApi.createType('XcmV1MultiLocation', {
                    parents: 1,
                    interior: karuraApi.createType('Junctions', {
                        X2: [
                            karuraApi.createType('XcmV1Junction', {
                                Parachain: karuraApi.createType('Compact<U32>', khalaParaId)
                            }),
                            karuraApi.createType('XcmV1Junction', {
                                AccountId32: {
                                    network: karuraApi.createType('XcmV0JunctionNetworkId', 'Any'),
                                    id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                                }
                            }),
                        ]
                    })
                })
            }),
            6000000000
        ).signAndSend(sender, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
    });
}

async function transferKARFromKhalaToKarura(khalaApi, sender, recipient, amount) {
    console.log(`Transfer KAR from Khala to Karura...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.xcmTransfer.transferAsset(
            khalaApi.createType('XtransferPalletsAssetsWrapperPalletXTransferAsset',
                khalaApi.createType('XcmV1MultiLocation', {
                    parents: 1,
                    interior: khalaApi.createType('Junctions', {
                        X2: [
                            khalaApi.createType('XcmV1Junction', {
                                Parachain: khalaApi.createType('Compact<U32>', karuraParaId)
                            }),
                            khalaApi.createType('XcmV1Junction', {
                                // 0x0080 is general key of KAR defined in karura runtime
                                GeneralKey: '0x0080'
                            }),
                        ]
                    })
                })
            ),
            khalaApi.createType('XcmV1MultiLocation', {
                parents: 1,
                interior: khalaApi.createType('Junctions', {
                    X2: [
                        khalaApi.createType('XcmV1Junction', {
                            Parachain: khalaApi.createType('Compact<U32>', karuraParaId)
                        }),
                        khalaApi.createType('XcmV1Junction', {
                            AccountId32: {
                                network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                                id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                            }
                        }),
                    ]
                })
            }),
            amount,
            6000000000
        ).signAndSend(sender, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
    });
}

// transfer assets except PHA between accounts on khala network
async function transferAssetsKhalaAccounts(khalaApi, sender, recipient, amount) {
    console.log(`Transfer KAR between accounts on khala network...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.assets.transfer(
            // 0 is assetid that KAR registered on khala network,
            // should confirm this with maintainer before run script
            0,
            recipient.address,
            amount,
        ).signAndSend(sender, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
    });
}

// simulate EVM => Khala, call Bridge.Deposit()
async function transferPhaFromEvmToKhala(khalaApi, bridge, sender, recipient, amount) {
    let khalaChainId = 1;
    let phaResourceId = '0x41ab283b2b268c9c99ddfe96ed5dfbfa3dcc1a2f5551a30049fea8484186f2eb';
    // dest is not Account public key any more.
    let dest = khalaApi.createType('XcmV1MultiLocation', {
        // parents = 0 means we send to xcm local network(e.g. Khala network here)
        parents: 0,
        interior: khalaApi.createType('Junctions', {
            X1: khalaApi.createType('XcmV1Junction', {
                    AccountId32: {
                        network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                        id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                    }
                })
        })
    }).toHex();

    let data  = '0x' +
        ethers.utils.hexZeroPad(ethers.BigNumber.from(amount.toString()).toHexString(), 32).substr(2) + 
        ethers.utils.hexZeroPad(ethers.utils.hexlify((dest.length - 2)/2), 32).substr(2) +
        dest.substr(2);

    await bridge.deposit(khalaChainId, phaResourceId, data);
}

// simulate EVM => Khala => Karura, call Bridge.Deposit()
async function transferPhaFromEvmToKarura(khalaApi, bridge, sender, recipient, amount) {
    let khalaChainId = 1;
    let phaResourceId = '0x41ab283b2b268c9c99ddfe96ed5dfbfa3dcc1a2f5551a30049fea8484186f2eb';
    let dest = khalaApi.createType('XcmV1MultiLocation', {
        // parents = 1 means we wanna send to other parachains or relaychain
        parents: 1,
        interior: khalaApi.createType('Junctions', {
            X2: [
                khalaApi.createType('XcmV1Junction', {
                    Parachain: khalaApi.createType('Compact<U32>', karuraParaId)
                }),
                khalaApi.createType('XcmV1Junction', {
                    AccountId32: {
                        network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                        id: '0x' + Buffer.from(recipient.publicKey).toString('hex'),
                    }
                }),
            ]
        })
    }).toHex();
    let data  = '0x' +
    ethers.utils.hexZeroPad(ethers.BigNumber.from(amount.toString()).toHexString(), 32).substr(2) + 
    ethers.utils.hexZeroPad(ethers.utils.hexlify((dest.length - 2)/2), 32).substr(2) +
    dest.substr(2);

    await bridge.deposit(khalaChainId, phaResourceId, data);
}

function dumpResourceId(khalaApi) {
    // PHA resourceId: 0x41ab283b2b268c9c99ddfe96ed5dfbfa3dcc1a2f5551a30049fea8484186f2eb
    let pha = khalaApi.createType('XcmV1MultiLocation', {
        parents: 1,
        interior: khalaApi.createType('Junctions', {
            X1: khalaApi.createType('XcmV1Junction', {
                    Parachain: khalaApi.createType('Compact<U32>', khalaParaId)
                })
        })
    }).toHex();

    console.log(`resource id of PHA: ${ethers.utils.keccak256(pha)}`);
}

async function main() {
    // create khala api
    const khalaEndpoint = process.env.ENDPOINT || 'ws://localhost:9944';
    const khalaProvider = new WsProvider(khalaEndpoint);
    const khalaApi = await ApiPromise.create({
        provider: khalaProvider,
    });

    // create karura api
    const karuraEndpoint = process.env.ENDPOINT || 'ws://localhost:9955';
    const karuraProvider = new WsProvider(karuraEndpoint);
    const karuraApi = await ApiPromise.create({
        provider: karuraProvider,
    });

    // create accounts
    const keyring = new Keyring({ type: 'sr25519' });
    const karuraAccount = keyring.addFromUri('//Alice');
    const khalaAccount = keyring.addFromUri('//Bob');
    const anotherKaruraAccount = keyring.addFromUri('//Charlie');
    // replace it with your owns, make sure private key used to sign transaction matchs address
    const evmSender = "0xA29D4E0F035cb50C0d78c8CeBb56Ca292616Ab20";

    // transfer 100 PHA from khalaAccount on khala network to karuraAccount on karura network
    await transferPHAFromKhalaToKarura(khalaApi, khalaAccount, karuraAccount, bn1e12.mul(new BN(100)));

    // now, karuraAccount has reserved 100 PHA on karura network(actually with small fee being deducted, so < 100)
    // transfer 50 PHA back from karuraAccount on khala network to khalaAccount on khala network
    await transferPHAFromKaruraToKhala(karuraApi, karuraAccount, khalaAccount, bn1e12.mul(new BN(50)));

    // transfer 100 KAR from karuraAccount on karura network to khalaAccount on khala network
    await transferKARFromKaruraToKhala(karuraApi, karuraAccount, khalaAccount, bn1e12.mul(new BN(100)));

    // now, khalaAccount has reserved 100 KAR on khala network(actually with small fee being deducted, so < 100)
    // transfer 50 KAR back from khalaAccount on khala network to karuraAccount on karura network
    await transferKARFromKhalaToKarura(khalaApi, khalaAccount, karuraAccount, bn1e12.mul(new BN(50)));

    // so far, khalaAccount has 50 KAR on khala network, transfer 10 KAR to another account on khala network.
    await transferAssetsKhalaAccounts(khalaApi, khalaAccount, anotherKaruraAccount, bn1e12.mul(new BN(10)));
    // browse https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/assets/balances check balance of KAR
    // note: ws endpoint should set correctly

    let privateKey = process.env.KEY;
    let provider = new ethers.providers.JsonRpcProvider('https://rinkeby.infura.io/v3/6d61e7957c1c489ea8141e947447405b');
    let ethereumWallet = new ethers.Wallet(privateKey, provider);
    let bridge = new ethers.Contract(bridgeAddressOnRinkeby, BridgeJson.abi, ethereumWallet);

    // transfer 10 PHA from rinkeby testnet to khala network
    // note: should confirm with maintainer whether the testnet relayer is running before run
    await transferPhaFromEvmToKhala(khalaApi, bridge, evmSender, khalaAccount, bn1e18.mul(new BN(10)));

    // transfer 10 PHA from rinkeby testnet to karura network
    // note: should confirm with maintainer whether the testnet relayer is running before run
    await transferPhaFromEvmToKarura(khalaApi, bridge, evmSender, karuraAccount, bn1e18.mul(new BN(10)));
}

main().catch(console.error).finally(() => process.exit());
