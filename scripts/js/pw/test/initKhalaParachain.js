const expect = require('chai');
const { getApiConnection, getAccount, alicePrivkey, bobPrivkey, charliePrivkey, davidPrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey, payeePrivkey, signerPrivkey } = require('../khala/khalaApi');
const { transferPha, setOverlordAccount, setPayeeAccount, pwCreateCollection, initializePhalaWorldClock, initializeRarityTypeCounts, setSignerAccount } = require('../util/tx');
const { token } = require("../pwUtils");

describe("Initialize Khala Parachain", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord, payee;

    before(async () => {
        api = await getApiConnection();
    });

    // Send PHA to Account from Alice
    it(`Send PHA to userAccounts`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        overlord = await getAccount(overlordPrivkey);
        const userAccounts = [overlord, bob, charlie, david, eve, ferdie];
        await transferPha(api, alice, userAccounts, token(20_000));
    });
    // Set Overlord account
    it(`Set Overlord Account`, async () => {
        alice = await getAccount(alicePrivkey);
        overlord = await getAccount(overlordPrivkey);
        await setOverlordAccount(api, alice, overlord);
    });
    // Set Payee account
    it(`Set Payee Account`, async () => {
        payee = await getAccount(payeePrivkey);
        overlord = await getAccount(overlordPrivkey);
        await setPayeeAccount(api, overlord, payee);
    });
    // Set Signer account
    it(`Set Signer Account`, async () => {
        signer = await getAccount(signerPrivkey);
        overlord = await getAccount(overlordPrivkey);
        await setSignerAccount(api, overlord, signer);
    });
    // Initialize PhalaWorld
    it(`Initialize PhalaWorld Clock`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await initializePhalaWorldClock(api, overlord);
    });
    // Create and Set PhalaWorld Collections
    it(`Create/Set Spirits Collection ID`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await pwCreateCollection(api, overlord, 'PhalaWorld Spirits Collection', null, 'PWSPRT', 'spirit');
    });
    it(`Create/Set Origin of Shells Collection ID`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await pwCreateCollection(api, overlord, 'PhalaWorld Origin of Shells Collection', null, 'PWOAS', 'originOfShell');
    });
    it(`Create/Set Shells Collection ID`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await pwCreateCollection(api, overlord, 'PhalaWorld Shells Collection', null, 'PWSHL', 'shell');
    });
    it(`Create/Set Shell Parts Collection ID`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await pwCreateCollection(api, overlord, 'PhalaWorld Shell Parts Collection', null, 'PWPRT', 'shellParts');
    });
    it(`Initialize PhalaWorld Rarity Inventory Count`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await initializeRarityTypeCounts(api, overlord);
    });
    after(() => {
        api.disconnect();
    });
})