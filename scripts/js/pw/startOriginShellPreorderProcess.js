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


// Start preorders origin of shells purchases
async function userPreorderOriginOfShell(khalaApi, preordersInfo) {
    for (const preorder of preordersInfo) {
        const index = preordersInfo.indexOf(preorder);
        const account = preorder.account;
        const race = recipient.race;
        const career = recipient.career;
        console.log(`[${index}]: Starting Preorder Origin of Shell account: ${account}, race: ${race}, career: ${career}...`);
        await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.preorderOriginOfShell(race, career), account);
    }
    console.log(`Preorder Origin of Shell...DONE`);
}

// Mint chosen preorders
async function mintChosenPreorders(khalaApi, root, chosenPreorders) {
    console.log(`Minting chosen preorders...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.mintChosenPreorders(chosenPreorders), account);
    console.log(`Minting chosen preorders...DONE`);
}

// Refund not chosen preorders
async function refundNotChosenPreorders(khalaApi, root, notChosenPreorders) {
    console.log(`Refunding not chosen preorders...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.refundNotChosenPreorders(notChosenPreorders), account);
    console.log(`Refunding not chosen preorders...DONE`);
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
    const userAccountPreordersOriginOfShellInfo = [
        {'account': alice, 'race': 'AISpectre', 'career': 'Web3Monk'},
        {'account': ferdie, 'race': 'Pandroid', 'career': 'RoboWarrior'},
        {'account': eve, 'race': 'XGene', 'career': 'TradeNegotiator'}
    ]

    const chosenPreorders = [0, 1];
    const notChosenPreorders = [2];

    // Disable the Whitelist sale
    await setStatusType(api, overlord, 'PurchasePrimeOriginOfShells', false);
    // Increase available NFTs for sale since the whitelist sale is over
    await api.tx.pwNftSale.updateRarityTypeCounts('Prime', 900, 50)
        .signAndSend(overlord);
    // Enable Preorder Process
    await setStatusType(api, overlord, 'PreorderOriginOfShells', true);
    // Preorder Prime Origin of Shell
    await userPreorderOriginOfShell(api, userAccountPreordersOriginOfShellInfo);
    // Disable the Preorders
    await setStatusType(api, overlord, 'PreorderOriginOfShells', false);
    // Mint chosen preorders
    await mintChosenPreorders(api, overlord, chosenPreorders);
    // Refund not chosen preorders
    await refundNotChosenPreorders(api, overlord, notChosenPreorders)
}

main().catch(console.error).finally(() => process.exit());