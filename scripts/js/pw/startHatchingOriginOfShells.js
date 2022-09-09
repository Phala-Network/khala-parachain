require('dotenv').config();
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { waitExtrinsicFinished, setStatusType, getNonce, token, waitTxAccepted} = require('./pwUtils');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

async function setIncubationProcess(khalaApi, overlord, status) {
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    return new Promise(async (resolve) => {
        console.log(`Setting CanStartIncubationStatus to ${status}...`);
        const unsub = await khalaApi.tx.pwIncubation.setCanStartIncubationStatus(status
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
        console.log(`Enabling the Incubation Process...Done`);
    });
}

async function hatchOriginOfShell(khalaApi, overlord, originOfShellsOwners) {
    const defaultMetadata = "";
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    for (const accountId of originOfShellsOwners) {
        const originOfShellCollectionId = await khalaApi.query.pwNftSale.originOfShellCollectionId();
        let nfts = [];
        if (originOfShellCollectionId.isSome) {
            const originOfShells = await khalaApi.query.uniques.account.entries(accountId.address, originOfShellCollectionId.unwrap());
            originOfShells
                .map(([key, _value]) =>
                    [key.args[0].toString(), key.args[1].toNumber(), key.args[2].toNumber()]
                ).forEach(([acct, collectionId, nftId]) => {
                nfts.push(nftId);
                console.log({
                    acct,
                    collectionId,
                    nftId,
                })
            })
        } else {
            throw new Error(
                'Origin of Shell Collection ID not configured'
            )
        }
        for (const nft of nfts) {
            console.log(`${accountId.address} hatching Origin of Shell for NFT ID: ${nft}...`);
            await khalaApi.tx.pwIncubation.hatchOriginOfShell(originOfShellCollectionId.unwrap(), nft, defaultMetadata).signAndSend(overlord, {nonce: nonceOverlord++});
            console.log(`${accountId.address} hatching Origin of Shell for NFT ID: ${nft}...Done`);
        }
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
    }
}

async function main() {
    const wsProvider = new WsProvider(endpoint);
    const api = await ApiPromise.create({
        provider: wsProvider,
    });

    const keyring = new Keyring({type: 'sr25519'});

    const alice = keyring.addFromUri(alicePrivkey);
    const bob = keyring.addFromUri(bobPrivkey);
    const ferdie = keyring.addFromUri(ferdiePrivkey);
    const overlord = keyring.addFromUri(overlordPrivkey);
    const charlie = keyring.addFromUri(charliePrivkey);
    const david = keyring.addFromUri(davidPrivkey);
    const eve = keyring.addFromUri(evePrivkey);

    const originOfShellsOwners = [alice, bob, charlie, david, eve, ferdie];
    // Use Overlord account to disable the incubation phase
    await setIncubationProcess(api, overlord, false);
    // Hatch all origin of shells
    await hatchOriginOfShell(api, overlord, originOfShellsOwners);

}

main().catch(console.error).finally(() => process.exit());