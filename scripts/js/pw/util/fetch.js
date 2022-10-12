
async function getCollectionsCount(khalaApi) {
    return (await khalaApi.query.rmrkCore.collectionIndex());
}

// export async function getBalance(khalaApi, account)

// export async function getOverlord(khalaApi)

// export async function getZeroDay(khalaApi)

// export async function getEra(khalaApi)

// export async function getClaimSpiritStatus(khalaApi)

// export async function getPurchaseRareOriginOfShellsStatus(khalaApi)

// export async function getPurchasePrimeOriginOfShellsStatus(khalaApi)

// export async function getPreorderOriginOfShellsStatus(khalaApi)

// export async function getLastDayOfSaleStatus(khalaApi)

async function getSpiritCollectionId(khalaApi) {
    console.log(`Querying Spirit Collection ID...`)
    let collectionId = await khalaApi.query.pwNftSale.spiritCollectionId();
    return collectionId;
}

async function getOriginOfShellCollectionId(khalaApi) {
    console.log(`Querying Origin of Shell Collection ID...`)
    let collectionId = await khalaApi.query.pwNftSale.originOfShellCollectionId();
    return collectionId;
}

// export async function getIsOriginOfShellsInventorySet(khalaApi)

// export async function getSpiritsMetadata(khalaApi)

// export async function getOriginOfShellsMetadata(khalaApi, raceType)

// export async function getNextNftId(khalaApi, collectionId)

// export async function getPreorderIndex(khalaApi)

// export async function getPreorder(khalaApi, preorderId)

// export async function getOriginOfShellsInventory(khalaApi, rarityType)

// export async function getFoodByOwners(khalaApi, account)

// export async function getOriginOfShellFoodStats(khalaApi, eraId, collectionId, nftId)

// export async function getOfficialHatchTime(khalaApi)

// export async function getCanStartIncubationStatus(khalaApi)

// export async function getHsOriginOfShellStartedIncubationStatus(khalaApi, collectionId, nftId)

async function getShellCollectionId(khalaApi) {
    console.log(`Querying Shell Collection ID...`)
    let collectionId = await khalaApi.query.pwIncubation.shellCollectionId();
    return collectionId;
}

async function getShellPartsCollectionId(khalaApi) {
    console.log(`Querying Shell Parts Collection ID...`)
    let collectionId = await khalaApi.query.pwIncubation.shellPartsCollectionId();
    return collectionId;
}

// export async function getOriginOfShellsChosenParts(khalaApi, collectionId, nftId)

module.exports = {
    getCollectionsCount,
    getSpiritCollectionId,
    getOriginOfShellCollectionId,
    getShellCollectionId,
    getShellPartsCollectionId
}