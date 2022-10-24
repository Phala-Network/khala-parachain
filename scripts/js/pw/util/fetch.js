
async function getCollectionsCount(khalaApi) {
    return (await khalaApi.query.rmrkCore.collectionIndex()).toNumber();
}

// export async function getBalance(khalaApi, account)

async function getOverlord(khalaApi) {
    return (await khalaApi.query.pwNftSale.overlord())
}

async function getZeroDay(khalaApi) {
    return (await khalaApi.query.pwNftSale.zeroDay())
}

async function getEra(khalaApi) {
    return (await khalaApi.query.pwNftSale.era()).toNumber();
}

async function getClaimSpiritStatus(khalaApi) {
    return (await khalaApi.query.pwNftSale.canClaimSpirits());
}

async function getPurchaseRareOriginOfShellsStatus(khalaApi) {
    return (await khalaApi.query.pwNftSale.canPurchaseRareOriginOfShells());
}

async function getPurchasePrimeOriginOfShellsStatus(khalaApi) {
    return (await khalaApi.query.pwNftSale.canPurchasePrimeOriginOfShells());
}

async function getPreorderOriginOfShellsStatus(khalaApi) {
    return (await khalaApi.query.pwNftSale.canPreorderOriginOfShells());
}

async function getLastDayOfSaleStatus(khalaApi) {
    return (await khalaApi.query.pwNftSale.lastDayOfSale());
}

async function getSpiritCollectionId(khalaApi) {
    console.log(`\tQuerying Spirit Collection ID...`)
    let collectionId = await khalaApi.query.pwNftSale.spiritCollectionId();
    return collectionId;
}

async function getOriginOfShellCollectionId(khalaApi) {
    console.log(`\tQuerying Origin of Shell Collection ID...`)
    let collectionId = await khalaApi.query.pwNftSale.originOfShellCollectionId();
    return collectionId;
}

async function getOwnedOriginOfShells(khalaApi, account, collectionId) {
    let nfts = [];
    const originOfShells = await khalaApi.query.uniques.account.entries(account.address, collectionId);
    originOfShells
        .map(([key, _value]) =>
            [key.args[0].toString(), key.args[1].toNumber(), key.args[2].toNumber()]
        ).forEach(([acct, colId, nftId]) => {
        nfts.push(nftId);
        console.log(`\tAdding [${acct}, ${colId}, ${nftId}]`);
    });
    return nfts;
}

async function getIsOriginOfShellsInventorySet(khalaApi) {
    return (await khalaApi.query.pwNftSale.isOriginOfShellsInventorySet());
}

async function getSpiritsMetadata(khalaApi) {
    return (await khalaApi.query.pwNftSale.spiritsMetadata());
}

async function getOriginOfShellsMetadata(khalaApi, raceType) {
    return (await khalaApi.query.pwNftSale.originOfShellsMetadata(raceType));
}

async function getNextNftId(khalaApi, collectionId) {
    return (await khalaApi.query.pwNftSale.nextNftId(collectionId)).toNumber();
}

async function getNextResourceId(khalaApi, collectionId, nftId) {
    return (await khalaApi.query.pwNftSale.nextResourceId(collectionId, nftId)).toNumber();
}

async function getPreorderIndex(khalaApi) {
    return (await khalaApi.query.pwNftSale.preorderIndex()).toNumber();
}

async function getPreorder(khalaApi, preorderId) {
    return (await khalaApi.query.pwNftSale.preorders(preorderId));
}

async function getOwnerHasPreorder(khalaApi, account) {
    return (await khalaApi.query.pwNftSale.ownerHasPreorder(account));
}

// Returns type NftSaleInfo for a RarityType and RaceType
async function getOriginOfShellsInventory(khalaApi, rarityType, raceType) {
    return (await khalaApi.query.pwNftSale.originOfShellsInventory(rarityType, raceType));
}

// Returns the latest FoodInfo for an Account
async function getFoodByOwners(khalaApi, account) {
    return (await khalaApi.query.pwIncubation.foodByOwners(account));
}

async function getOriginOfShellFoodStats(khalaApi, eraId, collectionId, nftId) {
    return (await khalaApi.query.pwIncubation.originOfShellsFoodStats(eraId, (collectionId, nftId))).toNumber();
}

async function getOfficialHatchTime(khalaApi) {
    return (await khalaApi.query.pwIncubation.officialHatchTime()).toNumber();
}

async function getCanStartIncubationStatus(khalaApi) {
    return (await khalaApi.query.pwIncubation.canStartIncubation());
}

async function getHasOriginOfShellStartedIncubationStatus(khalaApi, collectionId, nftId) {
    return (await khalaApi.query.pwIncubation.hasOriginOfShellStartedIncubation((collectionId, nftId)));
}

async function getShellCollectionId(khalaApi) {
    console.log(`\tQuerying Shell Collection ID...`)
    let collectionId = await khalaApi.query.pwIncubation.shellCollectionId();
    return collectionId;
}

async function getShellPartsCollectionId(khalaApi) {
    console.log(`\tQuerying Shell Parts Collection ID...`)
    let collectionId = await khalaApi.query.pwIncubation.shellPartsCollectionId();
    return collectionId;
}

async function getOriginOfShellsChosenParts(khalaApi, collectionId, nftId) {
    return (await khalaApi.query.pwIncubation.originOfShellsChosenParts((collectionId, nftId)));
}

async function getOwnedNftsInCollection(khalaApi, account, collectionId) {
    let nfts = [];
    const originOfShells = await khalaApi.query.uniques.account.entries(account.address, collectionId);
    originOfShells
        .map(([key, _value]) =>
            [key.args[0].toString(), key.args[1].toNumber(), key.args[2].toNumber()]
        ).forEach(([acct, colId, nftId]) => {
        nfts.push(nftId);
        console.log(`\tDetected owner:[${acct}] (collection_id, nft_id):[${colId}, ${nftId}]`);
    });
    return nfts;
}

async function getNftInfo(khalaApi, collectionId, nftId) {
    return (await khalaApi.query.rmrkCore.nfts(collectionId, nftId));
}

module.exports = {
    getCollectionsCount,
    getSpiritCollectionId,
    getOriginOfShellCollectionId,
    getShellCollectionId,
    getShellPartsCollectionId,
    getEra,
    getOwnedOriginOfShells,
    getOverlord,
    getZeroDay,
    getClaimSpiritStatus,
    getPurchaseRareOriginOfShellsStatus,
    getPurchasePrimeOriginOfShellsStatus,
    getPreorderOriginOfShellsStatus,
    getLastDayOfSaleStatus,
    getIsOriginOfShellsInventorySet,
    getSpiritsMetadata,
    getOriginOfShellsMetadata,
    getNextNftId,
    getNextResourceId,
    getPreorderIndex,
    getPreorder,
    getOwnerHasPreorder,
    getOriginOfShellsInventory,
    getFoodByOwners,
    getOriginOfShellFoodStats,
    getOfficialHatchTime,
    getCanStartIncubationStatus,
    getHasOriginOfShellStartedIncubationStatus,
    getOriginOfShellsChosenParts,
    getOwnedNftsInCollection,
    getNftInfo
}
