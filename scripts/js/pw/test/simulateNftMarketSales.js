const { getApiConnection, getAccount, alicePrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey, bobPrivkey,
    charliePrivkey, davidPrivkey
} = require('../khala/khalaApi');
const {setIncubationProcessStatus, hatchOriginOfShell, listNft, buyNft, sendNftToOwner, makeOfferOnNft,
    withdrawOfferOnNft, unlistNft, acceptOfferOnNft, failToSendNonTransferableNft
} = require("../util/tx");
const { getOwnedNftsInCollection, getShellCollectionId, getShellPartsCollectionId} = require("../util/fetch");
const {getNonTransferableNft} = require("../util/helpers");

describe("Simulate NFT Market Sales", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Alice Lists and Sells Shell NFT to Bob`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey)
        const shellCollectionId = await getShellCollectionId(api);
        expect(
            shellCollectionId.isSome,
            `Error: Shell Collection ID is not set`
        ).to.be.true;
        const nfts = await getOwnedNftsInCollection(api, alice, shellCollectionId.unwrap().toNumber());
        expect(
            nfts.length > 0,
            `Error: Alice does not own a Shell NFT`
        ).to.be.true;
        const nftId = nfts[0];
        await listNft(api, alice, shellCollectionId.unwrap(), nftId, 1000, null);
        await buyNft(api, bob, shellCollectionId.unwrap(), nftId, 1000);
    });
    let shellPartNftId;
    it(`Bob Sends Shell Part NFT to Self and Lists Shell Part NFT`, async () => {
        bob = await getAccount(bobPrivkey);
        const shellPartsCollectionId = await getShellPartsCollectionId(api);
        expect(
            shellPartsCollectionId.isSome,
            `Error: Shell Parts Collection ID is not set`
        ).to.be.true;
        const nfts = await getOwnedNftsInCollection(api, bob, shellPartsCollectionId.unwrap().toNumber());
        expect(
            nfts.length > 0,
            `Error: Bob does not own a Shell Parts NFT`
        ).to.be.true;
        shellPartNftId = nfts[0];
        await sendNftToOwner(api, bob, shellPartsCollectionId, shellPartNftId, bob);
        await listNft(api, bob, shellPartsCollectionId, shellPartNftId, 50, null);
    });
    it(`Charlie & David Make Offers on Bob's Shell Part Listed Then Charlie Withdraws Offer`, async () => {
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        const shellPartsCollectionId = await getShellPartsCollectionId(api);
        expect(
            shellPartsCollectionId.isSome,
            `Error: Shell Parts Collection ID is not set`
        ).to.be.true;
        await makeOfferOnNft(api, charlie, shellPartsCollectionId.unwrap(), shellPartNftId, 10, null);
        await makeOfferOnNft(api, david, shellPartsCollectionId.unwrap(), shellPartNftId, 45, 50);
        await withdrawOfferOnNft(api, charlie, shellPartsCollectionId.unwrap(), shellPartNftId);
    });
    it(`Bob Unlists Shell Part NFT Then Accepts David's Offer`, async () => {
        david = await getAccount(davidPrivkey);
        bob = await getAccount(bobPrivkey);
        const shellPartsCollectionId = await getShellPartsCollectionId(api);
        expect(
            shellPartsCollectionId.isSome,
            `Error: Shell Parts Collection ID is not set`
        ).to.be.true;
        await unlistNft(api, bob, shellPartsCollectionId.unwrap(), shellPartNftId);
        await acceptOfferOnNft(api, bob, shellPartsCollectionId.unwrap(), shellPartNftId, david);
    });
    it(`Bob Cannot Send Non-Transferable Shell Part`, async () => {
        bob = await getAccount(bobPrivkey);
        const shellPartsCollectionId = await getShellPartsCollectionId(api);
        expect(
            shellPartsCollectionId.isSome,
            `Error: Shell Parts Collection ID is not set`
        ).to.be.true;
        const nfts = await getOwnedNftsInCollection(api, bob, shellPartsCollectionId.unwrap().toNumber());
        expect(
            nfts.length > 0,
            `Error: Bob does not own a Shell Parts NFT`
        ).to.be.true;
        const nftId = await getNonTransferableNft(api, shellPartsCollectionId.unwrap(), nfts);
        expect(
            nftId > 0,
            `Error: Did not find a non-transferable Shell Part NFT`
        ).to.be.true;
        await failToSendNonTransferableNft(api, bob, shellPartsCollectionId.unwrap(), nftId, bob);
    });

    after(() => {
        api.disconnect();
    });
});

