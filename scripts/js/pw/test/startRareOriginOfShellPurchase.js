const { getApiConnection, getAccount, bobPrivkey, charliePrivkey, davidPrivkey, overlordPrivkey } = require('../khala/khalaApi');
const { addOriginOfShellsMetadata, setStatusType, usersPurchaseRareOriginOfShells } = require('../util/tx');

describe("Start Rare Origin of Shells Purchase", () => {
    let api;
    let bob, charlie, david, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Add Origin of Shells RarityType Metadata`, async () => {
        overlord = await getAccount(overlordPrivkey);
        const originOfShellsMetadataArr = [['Cyborg', 'ar://BS-NUyJWDKJ-CwTYLWZz6TpG0CbWVKUAXvdPQu-KimI'], ['AISpectre', 'ar://KR3ZIIcc_Q6_47sibLOJ5YoFwJZqT6C7aJkkUYbUWbU'], ['Pandroid', 'ar://AbMW_AEHo6WqoQdRJjOzRvZSR3QfYs_dvg7y6SlbilI'], ['XGene', 'ar://IzOXT_pER7487_RBpzGNOKNBGnDouN1mOcPXojE_Das']];
        await addOriginOfShellsMetadata(api, overlord, originOfShellsMetadataArr);
    });
    it(`Enable PurchaseRareOriginOfShells StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'PurchaseRareOriginOfShells', true);
    });
    it(`Simulate Rare Origin of Shell User Purchases`, async () => {
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        const userAccountsRareOriginOfShellInfo = [
            {'account': bob, 'rarity': 'Legendary', 'race': 'Cyborg', 'career': 'HackerWizard', 'amount': 15_001},
            {'account': charlie, 'rarity': 'Magic', 'race': 'Pandroid', 'career': 'RoboWarrior', 'amount': 10_001},
            {'account': david, 'rarity': 'Magic', 'race': 'XGene', 'career': 'TradeNegotiator', 'amount': 10_001}
        ];
        await usersPurchaseRareOriginOfShells(api, userAccountsRareOriginOfShellInfo);
    });
    after(() => {
        api.disconnect();
    });
})