require('dotenv').config();
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { token, waitTxAccepted, getNonce, setStatusType, checkUntil } = require('./pwUtils');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

// Start rare origin of shells purchases
async function userPreorderOriginOfShell(khalaApi, account, race, career) {
    let nonceAccount = await getNonce(khalaApi, account.address);
    return new Promise(async (resolve) => {
        console.log(`Starting Preorder Origin of Shell account: ${account}, race: ${race}, career: ${career}...`);
        const unsub = await khalaApi.tx.pwNftSale.preorderOriginOfShell(race, career).signAndSend(account, {nonce: nonceAccount++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
        console.log(`Preorder Origin of Shell...DONE`);

    });
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

    // Disable the Whitelist sale
    await setStatusType(api, overlord, 'PurchasePrimeOriginOfShells', false);
    // Increase available NFTs for sale since the whitelist sale is over
    await api.tx.pwNftSale.updateRarityTypeCounts('Prime', 900, 50)
        .signAndSend(overlord);
    // Enable Preorder Process
    await setStatusType(api, overlord, 'PreorderOriginOfShells', true);
    // Preorder Prime Origin of Shell
    await userPreorderOriginOfShell(api, alice, 'Cyborg', 'HackerWizard');
}

main().catch(console.error).finally(() => process.exit());