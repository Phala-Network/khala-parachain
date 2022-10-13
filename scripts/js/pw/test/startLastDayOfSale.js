const { getApiConnection, getAccount, alicePrivkey, bobPrivkey, charliePrivkey, davidPrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey } = require('../khala/khalaApi');
const { setStatusType, userPreorderOriginOfShell, mintChosenPreorders, refundNotChosenPreorders } = require('../util/tx');

describe("Start Last Day of Sale Preorder Process", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    // TODO: Query if PreorderOriginOfShells is enabled
    it(`Enable LastDayOfSale StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'LastDayOfSale', true);
    });
    it(`Last Day of Sale Preorder Prime Origin of Shells`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const userAccountLastDayPreordersOriginOfShellInfo = [
            {'account': alice, 'race': 'AISpectre', 'career': 'Web3Monk'},
            {'account': ferdie, 'race': 'Pandroid', 'career': 'RoboWarrior'},
            {'account': eve, 'race': 'XGene', 'career': 'TradeNegotiator'},
            {'account': bob, 'race': 'Cyborg', 'career': 'HackerWizard'},
            {'account': charlie, 'race': 'XGene', 'career': 'RoboWarrior'},
            {'account': david, 'race': 'Cyborg', 'career': 'TradeNegotiator'}
        ]
        await userPreorderOriginOfShell(api, userAccountLastDayPreordersOriginOfShellInfo);
    });
    it(`Disable LastDayOfSale StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'LastDayOfSale', false);
    });
    it(`Mint Chosen Preorders`, async () => {
        overlord = await getAccount(overlordPrivkey);
        // TODO: Maybe query the preorders & randomly choose instead
        const chosenPreorders = [2, 3, 4, 5];
        await mintChosenPreorders(api, overlord, chosenPreorders);
    });
    it(`Refund Not Chosen Preorders`, async () => {
        overlord = await getAccount(overlordPrivkey);
        // TODO: Maybe query the preorders & randomly choose instead
        const notChosenPreorders = [0, 1];
        await refundNotChosenPreorders(api, overlord, notChosenPreorders);
    });
    after(() => {
        api.disconnect();
    });
});