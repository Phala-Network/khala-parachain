const { getApiConnection, getAccount, alicePrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey } = require('../khala/khalaApi');
const { setStatusType, updateRarityTypeCounts, userPreorderOriginOfShell, mintChosenPreorders, refundNotChosenPreorders } = require('../util/tx');

describe("Start Origin of Shells Preorder Process", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Disable PurchasePrimeOriginOfShells StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'PurchasePrimeOriginOfShells', false);
    });
    it(`Update the Rarity Inventory Count`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await updateRarityTypeCounts(api, overlord, 'Prime', 900, 50);
    });
    it(`Enable PreorderOriginOfShells StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'PreorderOriginOfShells', true);
    });
    it(`Preorder Prime Origin of Shells`, async () => {
        alice = await getAccount(alicePrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const userAccountPreordersOriginOfShellInfo = [
            {'account': alice, 'race': 'AISpectre', 'career': 'Web3Monk'},
            {'account': ferdie, 'race': 'Pandroid', 'career': 'RoboWarrior'},
            {'account': eve, 'race': 'XGene', 'career': 'TradeNegotiator'}
        ];
        await userPreorderOriginOfShell(api, userAccountPreordersOriginOfShellInfo);
    });
    it(`Disable PreorderOriginOfShells StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'PreorderOriginOfShells', false);
    });
    it(`Mint Chosen Preorders`, async () => {
        overlord = await getAccount(overlordPrivkey);
        // TODO: Maybe query the preorders & randomly choose instead
        const chosenPreorders = [0, 1];
        await mintChosenPreorders(api, overlord, chosenPreorders);
    });
    it(`Refund Not Chosen Preorders`, async () => {
        overlord = await getAccount(overlordPrivkey);
        // TODO: Maybe query the preorders & randomly choose instead
        const notChosenPreorders = [2];
        await refundNotChosenPreorders(api, overlord, notChosenPreorders);
    });
    after(() => {
        api.disconnect();
    });
});