require('dotenv').config();

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsU8a } = require('@polkadot/util-crypto');
const BN = require('bn.js');
const ethers = require('ethers');
const BridgeJson = require('./Bridge.json');

const bn1e12 = new BN(10).pow(new BN(12));
const bn1e18 = new BN(10).pow(new BN(18));
const khalaParaId = 2004;
const karuraParaId = 2000;
const bifrostParaId = 2001;
const moonriverParaId = 2023;
const heikoParaId = 2085;
const basiliskParaId = 2090;

const bridgeAddressOnRinkeby = '0x0712Cf53B9fA1A33018d180a4AbcC7f1803F55f4';

function getPhaAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 0,
            interior: khalaApi.createType('Junctions', 'Here')
        })
    })
}

function getKarAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
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
    })
}

function getMovrAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', moonriverParaId)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        PalletInstance: 10
                    }),
                ]
            })
        })
    })
}

function getBncAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', bifrostParaId)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x0001'
                    }),
                ]
            })
        })
    })
}

function getZlkAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', bifrostParaId)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x0207'
                    }),
                ]
            })
        })
    })
}

function getBsxAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', basiliskParaId)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x00000000'
                    }),
                ]
            })
        })
    })
}

function getHkoAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', heikoParaId)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x484B4F'
                    }),
                ]
            })
        })
    })
}

// example evmAddress: 0xA29D4E0F035cb50C0d78c8CeBb56Ca292616Ab20
function generateMoonriverDest(khalaApi, evmAddress) {
    return khalaApi.createType('XcmV1MultiLocation', {
        parents: 1,
        interior: khalaApi.createType('Junctions', {
            X2: [
                khalaApi.createType('XcmV1Junction', {
                    Parachain: khalaApi.createType('Compact<U32>', moonriverParaId)
                }),
                khalaApi.createType('XcmV1Junction', {
                    AccountKey20: {
                        network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                        key: evmAddress,
                    }
                }),
            ]
        })
    })
}

async function transferPhaFromKhalaToKarura(khalaApi, sender, recipient, amount) {
    console.log(`Transfer PHA from Khala to Karura...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.xTransfer.transfer(
            khalaApi.createType('XcmV1MultiAsset', {
                id: getPhaAssetId(khalaApi),
                fun: khalaApi.createType('XcmV1MultiassetFungibility', {
                    Fungible: khalaApi.createType('Compact<U128>', amount)
                })
            }),
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

async function transferKarFromKhalaToKarura(khalaApi, sender, recipient, amount) {
    console.log(`Transfer KAR from Khala to Karura...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.xTransfer.transfer(
            khalaApi.createType('XcmV1MultiAsset', {
                id: getKarAssetId(khalaApi),
                fun: khalaApi.createType('XcmV1MultiassetFungibility', {
                    Fungible: khalaApi.createType('Compact<U128>', amount)
                })
            }),
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

// Make sure amount > bridge fee
async function transferPhaFromKhalaToEvm(khalaApi, sender, recipient, amount) {
    console.log(`Transfer PHA from Khala to EVM...`);
    return new Promise(async (resolve) => {
        const unsub = await khalaApi.tx.xTransfer.transfer(
            khalaApi.createType('XcmV1MultiAsset', {
                id: getPhaAssetId(khalaApi),
                fun: khalaApi.createType('XcmV1MultiassetFungibility', {
                    Fungible: khalaApi.createType('Compact<U128>', amount)
                })
            }),
            khalaApi.createType('XcmV1MultiLocation', {
                parents: 0,
                interior: khalaApi.createType('Junctions', {
                    X3: [
                        khalaApi.createType('XcmV1Junction', {
                            GeneralKey: '0x6362'    // string "cb"
                        }),
                        khalaApi.createType('XcmV1Junction', {
                            GeneralIndex: 0 // 0 is chainid of ethereum
                        }),
                        khalaApi.createType('XcmV1Junction', {
                            GeneralKey: recipient
                        }),
                    ]
                })
            }),
            null,   // No need to specify a certain weight if transfer will not through XCM
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

async function transferPhaFromKaruraToKhala(karuraApi, sender, recipient, amount) {
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
                                    id: '0x' + recipient.publicKey.toString('hex'),
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

async function transferKarFromKaruraToKhala(karuraApi, sender, recipient, amount) {
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
                                    id: '0x' + recipient.publicKey.toString('hex'),
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

// simulate EVM => Khala, call Bridge.Deposit()
async function transferPhaFromEvmToKhala(khalaApi, bridge, sender, recipient, amount) {
    let khalaChainId = 1;
    let phaResourceId = '0x00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6';
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

    const tx = await bridge.deposit(khalaChainId, phaResourceId, data);
    console.log(`Transfer PHA from EVM to Khala: ${tx.hash}`);

}

// simulate EVM => Khala => Karura, call Bridge.Deposit()
async function transferPhaFromEvmToKarura(khalaApi, bridge, sender, recipient, amount) {
    let khalaChainId = 1;
    let phaResourceId = '0x00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6';
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

function dumpResourceId(khalaApi, soloChainId) {
    // PHA resourceId: 0x00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6
    let pha = khalaApi.createType('XcmV1MultiLocation', {
        parents: 0,
        interior: khalaApi.createType('Junctions', "Here")
    }).toHex();

    let u8arid = blake2AsU8a(pha);
    u8arid[0] = soloChainId;
    let rid = '0x' + Buffer.from(u8arid).toString('hex');

    console.log(`resource id of PHA: ${rid}`);
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
    const evmRecipient = evmSender;

    // transfer 100 PHA from khalaAccount on khala network to karuraAccount on karura network
    await transferPhaFromKhalaToKarura(khalaApi, khalaAccount, karuraAccount, bn1e12.mul(new BN(100)));

    // now, karuraAccount has reserved 100 PHA on karura network(actually with small fee being deducted, so < 100)
    // transfer 50 PHA back from karuraAccount on khala network to khalaAccount on khala network
    await transferPhaFromKaruraToKhala(karuraApi, karuraAccount, khalaAccount, bn1e12.mul(new BN(50)));

    // transfer 100 KAR from karuraAccount on karura network to khalaAccount on khala network
    await transferKarFromKaruraToKhala(karuraApi, karuraAccount, khalaAccount, bn1e12.mul(new BN(100)));

    // now, khalaAccount has reserved 100 KAR on khala network(actually with small fee being deducted, so < 100)
    // transfer 50 KAR back from khalaAccount on khala network to karuraAccount on karura network
    await transferKarFromKhalaToKarura(khalaApi, khalaAccount, karuraAccount, bn1e12.mul(new BN(50)));

    await transferPhaFromKhalaToEvm(khalaApi, khalaAccount, evmRecipient, bn1e12.mul(new BN(50)));

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
