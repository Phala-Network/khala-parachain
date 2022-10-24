const { getApiConnection, getAccount, alicePrivkey, bobPrivkey, charliePrivkey, davidPrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey } = require('../khala/khalaApi');
const { addSpiritMetadata, setStatusType, usersClaimSpirits} = require('../util/tx');

describe("Start Spirit Claim Process", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Add Spirits Metadata`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await addSpiritMetadata(api, overlord, "ar://Q6N5cKjuLzihuiyVlpU-ANUM5ffKLwYYACKFN4mbNuw");
    });
    it(`Enable ClaimSpirits StatusType`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setStatusType(api, overlord, 'ClaimSpirits', true);
    });
    it(`Simulate Users' Spirit Claims`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const userAccounts = [alice, bob, charlie, david, eve, ferdie];
        await usersClaimSpirits(api, userAccounts);
    });
    after(() => {
        api.disconnect();
    });
})