require('dotenv').config();
require("@polkadot/api-augment");
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { setStatusType, token, waitExtrinsicFinished, waitTxAccepted, getNonce } = require('./pwUtils');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

// Create Whitelist for account and sign with Overlord
async function createWhitelistMessage(khalaApi, type, overlord, account) {
    const whitelistMessage = khalaApi.createType(type, {'account': account.address, 'purpose': 'BuyPrimeOriginOfShells'});
    console.log(`${whitelistMessage}`);
    return overlord.sign(whitelistMessage.toU8a());
}

// Start rare origin of shells purchases
async function usersPurchaseWhitelistOriginOfShells(khalaApi, root, recipientsInfo) {
    let nonceRoot = await getNonce(khalaApi, root.address);
    console.log(`Starting Whitelist Origin of Shells purchases...`);
    for (const recipient of recipientsInfo) {
        const index = recipientsInfo.indexOf(recipient);
        const account = recipient.account;
        const whitelistMessage = recipient.whitelistMessage;
        const race = recipient.race;
        const career = recipient.career;
        console.log(`[${index}]: Purchasing Prime Origin of Shell for owner: ${account.address}, whitelistMessage: ${whitelistMessage}, race: ${race}, career: ${career}`);
        await khalaApi.tx.balances.transfer(account.address, token(501)).signAndSend(root, {nonce: nonceRoot++});
        await waitTxAccepted(khalaApi, root.address, nonceRoot - 1);
        await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.buyPrimeOriginOfShell(whitelistMessage, race, career), account);
        console.log(`[${index}]: Prime Origin of Shells purchase...DONE`);
    }
}

async function main() {
    const wsProvider = new WsProvider(endpoint);
    const api = await ApiPromise.create({
        provider: wsProvider,
        types: {
            Purpose: {
                _enum: [
                    'RedeemSpirit',
                    'BuyPrimeOriginOfShells',
                ],
            },
            OverlordMessage: {
                account: 'AccountId',
                purpose: 'Purpose',
            }
        }
    });

    const keyring = new Keyring({type: 'sr25519'});

    const alice = keyring.addFromUri(alicePrivkey);
    const bob = keyring.addFromUri(bobPrivkey);
    const ferdie = keyring.addFromUri(ferdiePrivkey);
    const overlord = keyring.addFromUri(overlordPrivkey);
    const charlie = keyring.addFromUri(charliePrivkey);
    const david = keyring.addFromUri(davidPrivkey);
    const eve = keyring.addFromUri(evePrivkey);

    // Get Whitelist Message signed by Overlord for Alice, Ferdie & Eve
    const aliceWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, alice);
    const ferdieWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, ferdie);
    const eveWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, eve);
    const userAccountsWhitelistOriginOfShellInfo = [
        {'account': alice, 'whitelistMessage': aliceWlMessage, 'race': 'AISpectre', 'career': 'HardwareDruid'},
        {'account': ferdie, 'whitelistMessage': ferdieWlMessage, 'race': 'Cyborg', 'career': 'Web3Monk'},
        {'account': eve, 'whitelistMessage': eveWlMessage, 'race': 'XGene', 'career': 'RoboWarrior'}
    ];
    // Enable Whitelist purchases
    await setStatusType(api, overlord, 'PurchasePrimeOriginOfShells', true);
    // Purchase Prime Origin of Shell
    await usersPurchaseWhitelistOriginOfShells(api, overlord, userAccountsWhitelistOriginOfShellInfo);
}

main().catch(console.error).finally(() => process.exit());