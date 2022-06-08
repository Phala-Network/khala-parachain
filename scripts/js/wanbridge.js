require('dotenv').config();

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsU8a, cryptoWaitReady } = require('@polkadot/util-crypto');
const BN = require('bn.js');
const ethers = require('ethers');

const bn1e12 = new BN(10).pow(new BN(12));
const bn1e18 = new BN(10).pow(new BN(18));
const khalaParaId = 2004;
const karuraParaId = 2000;
const moonriverParaId = 2023;

function getPhaAssetId(khalaApi) {
    return khalaApi.createType('XcmV1MultiassetAssetId', {
        Concrete: khalaApi.createType('XcmV1MultiLocation', {
            parents: 0,
            interior: khalaApi.createType('Junctions', 'Here')
        })
    })
}


// simulate EVM => Khala, call Bridge.Deposit()
async function transferPhaFromEvmToKhala(khalaApi, sender, recipient, amount, finalization) {
    let smgId = '0x00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6';
    let tokenPair = 0;
    let srcChainId = 0;
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

    return new Promise(async (resolve, reject) => {

        const nonce = Number((await khalaApi.query.system.account(sender.address)).nonce);

        console.log(
            `--- Submitting extrinsic to handle fungible transfer (nonce: ${nonce}) ---`
        );
        const unsub = await khalaApi.tx.wanBridge.handleFungibleTransfer(
            smgId,
            tokenPair,
            srcChainId,
            amount,
            dest,
            '0x0'
        ).signAndSend(sender, { nonce: nonce, era: 0 }, (result) => {
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

    console.log(`Transfer PHA from EVM to Khala.`);
}


async function main() {
    // create khala api
    const khalaEndpoint = process.env.ENDPOINT || 'ws://localhost:9944';
    const khalaProvider = new WsProvider(khalaEndpoint);
    const khalaApi = await ApiPromise.create({
        provider: khalaProvider,
    });

    await cryptoWaitReady();

    // create accounts
    const keyring = new Keyring({ type: 'sr25519' });
    const Alice = keyring.addFromUri('//Alice');
    const Bob = keyring.addFromUri('//Bob');

    // transfer 20 PHA from rinkeby testnet to khala network
    // note: should confirm with maintainer whether the testnet relayer is running before run
    await transferPhaFromEvmToKhala(khalaApi, Alice, Bob, bn1e12.mul(new BN(20)), false);
}

main().catch(console.error).finally(() => process.exit());
