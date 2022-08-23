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


// Start preorder origin of shells purchases
async function userPreorderOriginOfShell(khalaApi, root, preordersInfo) {
    let nonceRoot = await getNonce(khalaApi, root.address);
    for (const preorder of preordersInfo) {
        const index = preordersInfo.indexOf(preorder);
        const account = preorder.account;
        const race = recipient.race;
        const career = recipient.career;
        console.log(`[${index}]: Starting Preorder Origin of Shell account: ${account}, race: ${race}, career: ${career}...`);
        await khalaApi.tx.balances.transfer(account.address, token(501)).signAndSend(root, {nonce: nonceRoot++});
        await waitTxAccepted(khalaApi, root.address, nonceRoot - 1);
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
    const userAccountLastDayPreordersOriginOfShellInfo = [
        {'account': alice, 'race': 'AISpectre', 'career': 'Web3Monk'},
        {'account': ferdie, 'race': 'Pandroid', 'career': 'RoboWarrior'},
        {'account': eve, 'race': 'XGene', 'career': 'TradeNegotiator'},
        {'account': bob, 'race': 'Cyborg', 'career': 'HackerWizard'},
        {'account': charlie, 'race': 'XGene', 'career': 'RoboWarrior'},
        {'account': david, 'race': 'Cyborg', 'career': 'TradeNegotiator'}
    ]

    const chosenPreorders = [2, 3, 4, 5];
    const notChosenPreorders = [0, 1];

    // Disable the Preorders sale
    await setStatusType(api, overlord, 'PreorderOriginOfShells', false);
    // Enable Preorder Process
    await setStatusType(api, overlord, 'LastDayOfSale', true);
    // Preorder Prime Origin of Shell
    await userPreorderOriginOfShell(api, overlord, userAccountLastDayPreordersOriginOfShellInfo);
    // Mint chosen preorders
    await mintChosenPreorders(api, overlord, chosenPreorders);
    // Refund not chosen preorders
    await refundNotChosenPreorders(api, overlord, notChosenPreorders)
}

main().catch(console.error).finally(() => process.exit());