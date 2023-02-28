require('dotenv').config();

const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const BN = require('bn.js');

const bn1e12 = new BN(10).pow(new BN(12));

const feeHandlerType = {
    BasicFeeHandler: "BasicFeeHandler",
    DynamicFeeHandler: "DynamicFeeHandler"
}

const supportedDestDomains = [
    {
        domainID: 1,
        chainID: 1
    },
    {
        domainID: 2,
        chainID: 2
    }
]

const ThreasuryAccount = "436H4jat8E3x6whTRudzJFuF1aFqkxmkYyQqNbh9C9EKhcz7";
const BridgeAccount = "42ujiHYJycjK7xVaoheTrzwDSMEjoVYxoeQsegY8DUgmgyY9";
const AssetRegistryAdmin = "5EYCAe5iixJKLJE5qkFfWJYxWd9ZsoBVKwBaiLrqY98qbmDC";

async function setBalance(api, who, value, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set balance of ${who} to ${value}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.balances.setBalance(who, value, 0))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function setFeeHandler(api, domainID, asset, feeHandlerType, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set fee handler on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaFeeHandlerRouter.setFeeHandler(domainID, asset, feeHandlerType))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function setFee(api, domainID, asset, amount, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set basic fee on domainID ${domainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBasicFeeHandler.setFee(domainID, asset, amount))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function setMpcAddress(api, mpcAddr, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to set MPC address. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBridge.setMpcAddress(mpcAddr))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function queryBridgePauseStatus(api, domainID) {
    let result = await api.query.sygmaBridge.isPaused(domainID);
    return result.toJSON()
}

async function registerAsset(api, location, id, properties, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to register asset: (nonce: ${nonce}) ---`
        );

        const unsub = await api.tx.sudo.sudo(api.tx.assetsRegistry.forceRegisterAsset(location, id, getAssetProperties(api, properties)))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function enableSygmaBridge(api, id, resource_id, domain_id, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to register asset: (nonce: ${nonce}) ---`
        );

        const unsub = await api.tx.sudo.sudo(api.tx.assetsRegistry.forceEnableSygmabridge(id, resource_id, domain_id, false, "0x0"))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function mintAsset(api, id, recipient, amount, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to mint asset to ${recipient}: (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo.sudo(api.tx.assetsRegistry.forceMint(id, recipient, amount))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

async function registerDomain(api, domainID, chainID, finalization, sudo) {
    return new Promise(async (resolve, reject) => {
        const nonce = Number((await api.query.system.account(sudo.address)).nonce);

        console.log(
            `--- Submitting extrinsic to register domainID ${domainID} with chainID ${chainID}. (nonce: ${nonce}) ---`
        );
        const unsub = await api.tx.sudo
            .sudo(api.tx.sygmaBridge.registerDomain(domainID, chainID))
            .signAndSend(sudo, {nonce: nonce, era: 0}, (result) => {
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

function getUSDCLocation(api) {
    return api.createType('XcmV1MultiLocation', {
        parents: 1,
        interior: api.createType('Junctions', {
            X2: [
                api.createType('XcmV1Junction', {
                    Parachain: api.createType('Compact<U32>', 2004)
                }),
                api.createType('XcmV1Junction', {
                    // hex of "usdc"
                    GeneralKey: '0x75736463'
                }),
            ]
        })
    })
}

function getUSDCAssetId(api) {
    return api.createType('XcmV1MultiassetAssetId', {
        Concrete: getUSDCLocation(api)
    })
}

function getNativeAssetId(api) {
    return api.createType('XcmV1MultiassetAssetId', {
        Concrete: api.createType('XcmV1MultiLocation', {
            parents: 0,
            interior: api.createType('Junctions', 'Here')
        })
    })
}

function getAssetProperties(api, properties) {
    return api.createType('AssetsRegistryAssetProperties', {
        name: properties.name,
        symbol: properties.symbol,
        decimals: properties.decimals
    })
}

async function main() {
    const sygmaPalletProvider = new WsProvider(process.env.PALLETWSENDPOINT || 'ws://127.0.0.1:9944');
    const api = await ApiPromise.create({
        provider: sygmaPalletProvider,
    });

    await cryptoWaitReady();
    const keyring = new Keyring({type: 'sr25519'});
    const sudo = keyring.addFromUri('//Alice');
    const basicFeeAmount = bn1e12.mul(new BN(1)); // 1 * 10 ** 12
    const mpcAddr = process.env.MPCADDR || '0x1c5541A79AcC662ab2D2647F3B141a3B7Cdb2Ae4';

    // set up MPC address
    await setMpcAddress(api, mpcAddr, false, sudo);

    // register dest domains
    for (const domain of supportedDestDomains) {
        await registerDomain(api, domain.domainID, domain.chainID, false, sudo);
    }

    // set fee for native asset for domains
    for (const domain of supportedDestDomains) {
        await setFeeHandler(api, domain.domainID, getNativeAssetId(api), feeHandlerType.BasicFeeHandler, false, sudo)
        await setFee(api, domain.domainID, getNativeAssetId(api), basicFeeAmount, false, sudo);
    }

    // create USDC test asset (foreign asset)
    const usdcAssetID = 0;
    const usdcAdmin = sudo.address;
    const usdcName = "USDC test asset";
    const usdcSymbol = "USDC";
    const usdcDecimal = 12;
    const usdcResourceID = "0x0000000000000000000000000000000000000000000000000000000000000300";
    // transfer some native asset to AssetRegistryAdmin to reserve for asset creation
    await setBalance(api, AssetRegistryAdmin, bn1e12.mul(new BN(10)), false, sudo); // set balance to 10 native asset
    await registerAsset(api, getUSDCLocation(api), usdcAssetID, {name: usdcName, symbol: usdcSymbol, decimals: usdcDecimal}, false, sudo);
    await mintAsset(api, usdcAssetID, usdcAdmin, bn1e12.mul(new BN(10000)), false, sudo); // mint 10000 USDC to Alice

    // set fee for USDC for domains
    for (const domain of supportedDestDomains) {
        await setFeeHandler(api, domain.domainID, getUSDCAssetId(api), feeHandlerType.BasicFeeHandler, false, sudo)
        await setFee(api, domain.domainID, getUSDCAssetId(api), basicFeeAmount, false, sudo);
        // Enable sygmabridge
        await enableSygmaBridge(api, usdcAssetID, usdcResourceID, domain.domainID, false, sudo)
    }

    // transfer some native asset to FeeReserveAccount as Existential Deposit(aka ED)
    await setBalance(api, ThreasuryAccount, bn1e12.mul(new BN(1)), false, sudo); // set balance to 1 native asset
    // transfer some native asset to BridgeAccount as initial liquidity
    await setBalance(api, BridgeAccount, bn1e12.mul(new BN(10000)), false, sudo); // set balance to 10000 native asset

    // bridge should be unpaused by the end of the setup
    for (const domain of supportedDestDomains) {
        if (!await queryBridgePauseStatus(api, domain.domainID)) console.log(`DestDomainID: ${domain.domainID} is ready✅`);
    }

    console.log('🚀 Sygma substrate pallet setup is done! 🚀');

    // It is unnecessary to set up access segregator here since ALICE will be the sudo account and all methods with access control logic are already setup in this script.
    // so that on Relayer, E2E test only cases about public extrinsic such as deposit, executionProposal, retry .etc
}

main().catch(console.error).finally(() => process.exit());
