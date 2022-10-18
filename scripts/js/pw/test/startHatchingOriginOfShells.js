const { getApiConnection, getAccount, alicePrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey, bobPrivkey,
    charliePrivkey, davidPrivkey
} = require('../khala/khalaApi');
const { setIncubationProcessStatus, hatchOriginOfShell } = require('../util/tx');

describe("Start Shell NFT Hatching Process", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Disable Incubation Process`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setIncubationProcessStatus(api, overlord, false);
    });
    it(`Start Hatching Origin of Shells into Shell NFTs`, async () => {
        overlord = await getAccount(overlordPrivkey);
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const originOfShellsOwners = [alice, bob, charlie, david, eve, ferdie];
        // Set chosen part for NFT ID 0
        await hatchOriginOfShell(api, overlord, originOfShellsOwners);
    });

    after(() => {
        api.disconnect();
    });
});