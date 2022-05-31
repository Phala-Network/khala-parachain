require('dotenv').config();

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady } = require('@polkadot/util-crypto');
const BN = require('bn.js');

const bn1e12 = new BN(10).pow(new BN(12));
const bridgeAccount = '5EYCAe5iixJKLJE5vokZcdJwS4ZpFU23Ged95YDBznC789dM';
const assetRegistryAccount = '5EYCAe5iixJKLJE5qkFfWJYxWd9ZsoBVKwBaiLrqY98qbmDC';
const relayerAccount = '5DV9WVCfAejeGJgbRVkXqWAqAMyqzjf7tTVSDAZGWQG69Dhz';

async function setBalance(api, who, value, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
		const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
                `--- Submitting extrinsic to set balance of ${who} to ${value}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
        .sudo(api.tx.balances.setBalance(who, value, 0))
			.signAndSend(sudo, { nonce: nonce, era: 0 }, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                        console.log(
                                `Transaction included at blockHash ${result.status.asInBlock}`
                        );
                        if (finalization) {
                                console.log('Waiting for finalization...');
                        } else {
                                unsub();
                                resolve();
                        }
                } else if (result.status.isFinalized) {
                        console.log(
                                `Transaction finalized at blockHash ${result.status.asFinalized}`
                        );
                        unsub();
                        resolve();
                } else if (result.isError) {
                        console.log(`Transaction Error`);
                        reject(`Transaction Error`);
                }
        });
    });
}

async function registerAsset(api, location, id, properties, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
		const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
                `--- Submitting extrinsic to register asset: ${location}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
        .sudo(api.tx.assetsRegistry.forceRegisterAsset(location, id, properties))
			.signAndSend(sudo, { nonce: nonce, era: 0 }, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                        console.log(
                                `Transaction included at blockHash ${result.status.asInBlock}`
                        );
                        if (finalization) {
                                console.log('Waiting for finalization...');
                        } else {
                                unsub();
                                resolve();
                        }
                } else if (result.status.isFinalized) {
                        console.log(
                                `Transaction finalized at blockHash ${result.status.asFinalized}`
                        );
                        unsub();
                        resolve();
                } else if (result.isError) {
                        console.log(`Transaction Error`);
                        reject(`Transaction Error`);
                }
        });
    });
}

async function enableChainbridge(api, id, chainId, isMintable, metadata, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
		const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
                `--- Submitting extrinsic to enable chainbridge for asset ${id}, chain: ${chainId}. (nonce: ${nonce}) ---`
        );

        const unsub = await api.tx.sudo
        .sudo(api.tx.assetsRegistry.forceEnableChainbridge(id, chainId, isMintable, metadata))
			.signAndSend(sudo, { nonce: nonce, era: 0 }, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                        console.log(
                                `Transaction included at blockHash ${result.status.asInBlock}`
                        );
                        if (finalization) {
                                console.log('Waiting for finalization...');
                        } else {
                                unsub();
                                resolve();
                        }
                } else if (result.status.isFinalized) {
                        console.log(
                                `Transaction finalized at blockHash ${result.status.asFinalized}`
                        );
                        unsub();
                        resolve();
                } else if (result.isError) {
                        console.log(`Transaction Error`);
                        reject(`Transaction Error`);
                }
        });
    });
}

async function configChainBridge(api, relayer, fee, chainId, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
		const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
                `--- Submitting extrinsic to config chainbridge. (nonce: ${nonce}) ---`
        );

        // Add relayer
        const addRelayer = api.tx.sudo.sudo(api.tx.chainBridge.addRelayer(relayer));
        const setFee = api.tx.sudo.sudo(api.tx.chainBridge.updateFee(fee, 0, chainId));
        const whitelistChain = api.tx.sudo.sudo(api.tx.chainBridge.whitelistChain(chainId));

        const unsub = await api.tx.utility.batch([addRelayer, setFee, whitelistChain])
			.signAndSend(sudo, { nonce: nonce, era: 0 }, (result) => {
                console.log(`Current status is ${result.status}`);
                if (result.status.isInBlock) {
                        console.log(
                                `Transaction included at blockHash ${result.status.asInBlock}`
                        );
                        if (finalization) {
                                console.log('Waiting for finalization...');
                        } else {
                                unsub();
                                resolve();
                        }
                } else if (result.status.isFinalized) {
                        console.log(
                                `Transaction finalized at blockHash ${result.status.asFinalized}`
                        );
                        unsub();
                        resolve();
                } else if (result.isError) {
                        console.log(`Transaction Error`);
                        reject(`Transaction Error`);
                }
        });
    });
}

async function main() {
    // Create khala api
    const khalaEndpoint = process.env.ENDPOINT || 'ws://localhost:9944';
    const khalaProvider = new WsProvider(khalaEndpoint);
    const khalaApi = await ApiPromise.create({
        provider: khalaProvider,
    });

	await cryptoWaitReady();
	const keyring = new Keyring({ type: 'sr25519' });
	const sudo = keyring.addFromUri('//Alice');

    // Set balance;
	await setBalance(khalaApi, bridgeAccount, bn1e12.mul(new BN(10000)), false, sudo);
	await setBalance(khalaApi, assetRegistryAccount, bn1e12.mul(new BN(10)), false, sudo);
	await setBalance(khalaApi, relayerAccount, bn1e12.mul(new BN(10)), false, sudo);

    // Register KSM
    await registerAsset(
        khalaApi,
        khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', 'Here')
        }),
        0,
        khalaApi.createType('AssetsRegistryAssetProperties', {
            name: 'KSM',
            symbol: 'KSM',
            decimals: 12
        }),
		false,
		sudo
    );

    // Register KAR
    await registerAsset(
        khalaApi,
        khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', 2000)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x0080'
                    }),
                ]
            })
        }),
        1,
        khalaApi.createType('AssetsRegistryAssetProperties', {
            name: 'KAR',
            symbol: 'KAR',
            decimals: 12
        }),
		false,
		sudo
    );

    // Register BNC
    await registerAsset(
        khalaApi,
        khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', 2001)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x0001'
                    }),
                ]
            })
        }),
        2,
        khalaApi.createType('AssetsRegistryAssetProperties', {
            name: 'BNC',
            symbol: 'BNC',
            decimals: 12
        }),
		false,
		sudo
    );

    // Register ZLK
    await registerAsset(
        khalaApi,
        khalaApi.createType('XcmV1MultiLocation', {
            parents: 1,
            interior: khalaApi.createType('Junctions', {
                X2: [
                    khalaApi.createType('XcmV1Junction', {
                        Parachain: khalaApi.createType('Compact<U32>', 2001)
                    }),
                    khalaApi.createType('XcmV1Junction', {
                        GeneralKey: '0x0207'
                    }),
                ]
            })
        }),
        3,
        khalaApi.createType('AssetsRegistryAssetProperties', {
            name: 'ZLK',
            symbol: 'ZLK',
            decimals: 18
        }),
		false,
		sudo
	);

	// Register aUSD
	await registerAsset(
		khalaApi,
		khalaApi.createType('XcmV1MultiLocation', {
			parents: 1,
			interior: khalaApi.createType('Junctions', {
				X2: [
					khalaApi.createType('XcmV1Junction', {
						Parachain: khalaApi.createType('Compact<U32>', 2000)
					}),
					khalaApi.createType('XcmV1Junction', {
						GeneralKey: '0x0081'
					}),
				]
			})
		}),
		4,
		khalaApi.createType('AssetsRegistryAssetProperties', {
			name: 'Acala USD',
			symbol: 'aUSD',
			decimals: 12
		}),
		false,
		sudo
	);

	// Register BSX
	await registerAsset(
		khalaApi,
		khalaApi.createType('XcmV1MultiLocation', {
			parents: 1,
			interior: khalaApi.createType('Junctions', {
				X2: [
					khalaApi.createType('XcmV1Junction', {
						Parachain: khalaApi.createType('Compact<U32>', 2090)
					}),
					khalaApi.createType('XcmV1Junction', {
						GeneralKey: '0x00000000'
					}),
				]
			})
		}),
		5,
		khalaApi.createType('AssetsRegistryAssetProperties', {
			name: 'Basilisk',
			symbol: 'BSX',
			decimals: 12
		}),
		false,
		sudo
	);

    // Enable Chainbridge of ZLK to Moonriver EVM
	await enableChainbridge(khalaApi, 3, 2, false, 0x00, false, sudo);

    // Config ChainBridge of Ethereum(chainId 0)
	await configChainBridge(khalaApi, relayerAccount, bn1e12.mul(new BN(300)), 0, false, sudo);

    // Config ChainBridge of Moonriver EVM(chainId 2)
	await configChainBridge(khalaApi, relayerAccount, bn1e12.mul(new BN(1)), 2, false, sudo);

    console.log('🚀 Setup Subbridge done 🚀');
}

main().catch(console.error).finally(() => process.exit());
