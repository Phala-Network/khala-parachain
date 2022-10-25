const { getApiConnection, getAccount, alicePrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey } = require('../khala/khalaApi');
const { usersPurchaseWhitelistOriginOfShells, setStatusType} = require('../util/tx');
const { createWhitelistMessage } = require('../util/helpers');

describe("Start Origin of Shell Whitelist Purchase Process", () => {
    let api;
    let alice, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Enable PurchasePrimeOriginOfShells StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'PurchasePrimeOriginOfShells', true);
    });
    it(`Simulate Whitelist Origin of Shell Purchases`, async () => {
        alice = await getAccount(alicePrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const aliceWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, alice);
        const ferdieWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, ferdie);
        const eveWlMessage = await createWhitelistMessage(api, 'OverlordMessage', overlord, eve);
        const userAccountsWhitelistOriginOfShellInfo = [
            {'account': alice, 'whitelistMessage': aliceWlMessage, 'race': 'AISpectre', 'career': 'HardwareDruid'},
            {'account': ferdie, 'whitelistMessage': ferdieWlMessage, 'race': 'Cyborg', 'career': 'Web3Monk'},
            {'account': eve, 'whitelistMessage': eveWlMessage, 'race': 'XGene', 'career': 'RoboWarrior'}
        ];
        await usersPurchaseWhitelistOriginOfShells(api, userAccountsWhitelistOriginOfShellInfo);
    });
    after(() => {
        api.disconnect();
    });
})