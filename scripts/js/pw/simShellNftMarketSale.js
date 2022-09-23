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

// List NFT for sale. When a NFT is listed, the NFT sets the Lock StorageValue for the NFT to true. This will prevent the
// NFT from being transferred or modified during the time the Lock is enabled.
async function listNft(khalaApi, owner, collectionId, nftId, amount, maybeExpires) {
    console.log(`Listing Shell NFT ID [${collectionId}, ${nftId}] for sale...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.list(collectionId, nftId, amount, maybeExpires), owner);
    console.log(`Listing Shell NFT ID [${collectionId}, ${nftId}] for sale...DONE`);
}

// Buy NFT for sale. The buyer can set an Option<maybeAmount> to ensure that the buyer is buying at the expected price
// of the listed NFT. Once the purchase passes permissions checks, the NFT is unlocked, money is transferred to the old
// owner, the NFT is sent to the buyer & an event is emitted named TokenSold {owner, buyer, collection_id, nft_id, price}
async function buyNft(khalaApi, buyer, collectionId, nftId, maybeAmount) {
    console.log(`Buying Shell NFT ID [${collectionId}, ${nftId}]...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.buy(collectionId, nftId, maybeAmount), buyer);
    console.log(`Buying Shell NFT ID [${collectionId}, ${nftId}]...DONE`);
}

// Unlist NFT for sale. The owner can unlist the NFT listed in the market. This will set the Lock to false & remove the
// listed NFT from ListedNfts storage.
async function unlistNft(khalaApi, owner, collectionId, nftId) {
    console.log(`Unlisting Shell NFT ID [${collectionId}, ${nftId}]...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.unlist(collectionId, nftId), owner);
    console.log(`Unlisting Shell NFT ID [${collectionId}, ${nftId}]...DONE`);
}

// Make Offer for NFT. The Offerer will make an offer on a NFT at a set amount with an optional BlockNumber defined for
// when the offer expires. When the owner sees this offer in Offers storage then the owner can accept the offer.
async function makeOfferOnNft(khalaApi, offerer, collectionId, nftId, amount, maybeExpires) {
    console.log(`Make offer on Shell NFT ID [${collectionId}, ${nftId}]...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.makeOffer(collectionId, nftId, amount, maybeExpires), offerer);
    console.log(`Make offer on Shell NFT ID [${collectionId}, ${nftId}]...DONE`);
}

// Accept Offer for NFT. The owner can accept an active offer in storage for a NFT. This will remove the offer from storage,
// unreserve the offer from the offerer then transfer the funs to the current owner, then the NFT is sent to the offerer.
// This triggers an event OfferAccepted {owner, buyer, collection_id, nft_id}
async function acceptOfferOnNft(khalaApi, owner, collectionId, nftId, offerer) {
    console.log(`Accept offer on Shell NFT ID [${collectionId}, ${nftId}]...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.acceptOffer(collectionId, nftId, offerer), owner);
    console.log(`Accept offer on Shell NFT ID [${collectionId}, ${nftId}]...DONE`);
}

// Withdraw Offer for NFT. The offerer can withdraw an offer on a NFT which will remove the offer from the Offers in
// storage and the offer will no longer be able to be accepted.
async function withdrawOfferOnNft(khalaApi, offerer, collectionId, nftId) {
    console.log(`Withdraw offer on Shell NFT ID [${collectionId}, ${nftId}]...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.rmrkMarket.withdrawOffer(collectionId, nftId), offerer);
    console.log(`Withdraw offer on Shell NFT ID [${collectionId}, ${nftId}]...DONE`);
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

    // Alice list Shell NFT
    await listNft(api, alice, 2, 0, 10, null);
    // Bob buys listed Shell NFT
    await buyNft(api, bob, 2, 0, 10);
    // Bob list Shell NFT
    await listNft(api, bob, 2, 0, 100, null);
    // Bob unlist Shell NFT
    await unlistNft(api, bob, 2, 0);
    // Charlie fails to buy Shell NFT
    await buyNft(api, charlie, 2, 0, null);
    // Charlie makes an offer on Shell NFT
    await makeOfferOnNft(api, charlie, 2, 0, 200, null);
    // David makes an offer on Shell NFT
    await makeOfferOnNft(api, david, 2, 0, 150, null);
    // Bob accepts offer from Charlie
    await acceptOfferOnNft(api, bob, 2, 0, charlie);
    // David withdraws offer on Shell NFT
    await withdrawOfferOnNft(api, david, 2, 0);
}

main().catch(console.error).finally(() => process.exit());
