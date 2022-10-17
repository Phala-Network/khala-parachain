const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const { getCollectionsCount, getOriginOfShellCollectionId, getOwnedOriginOfShells } = require('./fetch');
const { token, extractTxResult, waitExtrinsicFinished, getNonce, getCollectionType, waitTxAccepted } = require('./helpers');

chai.use(chaiAsPromised);
const expect = chai.expect;

// Transfer balance to an account
async function transferPha(khalaApi, sender, recipients, amount) {
    let senderNonce = await getNonce(khalaApi, sender.address);
    return new Promise(async (resolve) => {
        console.log(`\tStarting transfers...`);
        for (const recipient of recipients) {
            const index = recipients.indexOf(recipient);
            console.log(`\t[${index}]: Transferring ${amount.toString()} PHA from ${sender.address} to ${recipient.address}`);
            const unsub = await khalaApi.tx.balances.transfer(recipient.address, amount).signAndSend(sender, {nonce: senderNonce++}, (result) => {
                if (result.status.isInBlock) {
                    console.log(`\tTransaction included at blockHash ${result.status.asInBlock}`);
                } else if (result.status.isFinalized) {
                    console.log(`\tTransaction finalized at blockHash ${result.status.asFinalized}`);
                    unsub();
                    resolve();
                }
            });
            console.log(`\t[${index}]: Transferring...DONE`);
        }
        await waitTxAccepted(khalaApi, sender.address, senderNonce - 1);
    });
}

// Set Overlord Account with Sudo account
async function setOverlordAccount(khalaApi, sender, newOverlord) {
    const tx = khalaApi.tx.sudo.sudo(
        khalaApi.tx.pwNftSale.setOverlord(newOverlord.address)
    );
    const result = await waitExtrinsicFinished(khalaApi, tx, sender);
    //const result = extractTxResult(events);
    expect(
        result,
        `Error: could not set new overlord[${newOverlord.address}]`
    ).to.be.true;
}

// Initialize PhalaWorld Clock
async function initializePhalaWorldClock(khalaApi, overlord) {
    const tx = khalaApi.tx.pwNftSale.initializeWorldClock();
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    //const result = extractTxResult(events);
    expect(
        result,
        `Error: could not set PhalaWorld Clock`
    ).to.be.true;
}

async function pwCreateCollection(
    khalaApi,
    overlord,
    metadata,
    maxOptional,
    symbol,
    collectionType // spirit, originOfShell, shell, shellParts
) {
    const oldCollectionCount = await getCollectionsCount(khalaApi);
    let expectCollectionType = await getCollectionType(khalaApi, collectionType);
    expect(
        expectCollectionType
    ).to.be.not.equal(
    -1,
    `Error: invalid collectionType[${collectionType}]`
    );
    expect(
        expectCollectionType.isSome,
`Error: collectionType[${collectionType}] is already set`
    ).to.be.false;
    const tx = khalaApi.tx.pwNftSale.pwCreateCollection(
        metadata,
        maxOptional,
        symbol
    );
    // TODO: Handle Events if we want to expand functionality later
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    //const result = extractTxResult(events);
    expect(
        result,
        `Error: create collectionId[${oldCollectionCount}]`
    ).to.be.true;
    const newCollectionCount = await getCollectionsCount(khalaApi);
    expect(newCollectionCount).to.be.equal(
        oldCollectionCount + 1,
        "Error: NFT collection count should increase"
    );
    await pwSetCollection(
        khalaApi,
        overlord,
        oldCollectionCount,
        collectionType
    );
}

async function pwSetCollection(
    khalaApi,
    overlord,
    collectionId,
    collectionType
) {
    let tx = null;
    if (collectionType === "spirit") {
        tx = khalaApi.tx.pwNftSale.setSpiritCollectionId(collectionId);
    } else if (collectionType === "originOfShell") {
        tx =  khalaApi.tx.pwNftSale.setOriginOfShellCollectionId(collectionId);
    } else if (collectionType === "shell") {
        tx = khalaApi.tx.pwIncubation.setShellCollectionId(collectionId);
    } else if (collectionType === "shellParts") {
        tx = khalaApi.tx.pwIncubation.setShellPartsCollectionId(collectionId);
    } else {
        expect(
            tx
        ).to.be.not.equal(
            null,
            `Error: Incorrect collectionType[${collectionType}]`
        );
    }
    let result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    //let result = extractTxResult(events);
    expect(
        result,
        `ErrorError: collectionType[${collectionType}] not set for collectionId[${collectionId}]`
    ).to.be.true;
    let expectCollectionType = await getCollectionType(khalaApi, collectionType);
    expect(
        expectCollectionType.isSome,
        `Error: collectionType[${collectionType}] not set for collectionId[${collectionId}]`
    ).to.be.true;
    expect(expectCollectionType.unwrap().toNumber()).to.be.equal(
        collectionId,
        `Error: collectionId[${collectionId} does not match expected collectionId[${expectCollectionType.unwrap}]`
    );
}

// Set PhalaWorld Rarity Inventory Count
async function initializeRarityTypeCounts(khalaApi, overlord) {
    const tx = khalaApi.tx.pwNftSale.initRarityTypeCounts();
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    //const result = extractTxResult(events);
    expect(
        result,
        `Error: could not set PhalaWorld RarityType Inventory Count`
    ).to.be.true;
}

// Set StatusType for
async function setStatusType(khalaApi, overlord, statusType, status) {
    const tx = khalaApi.tx.pwNftSale.setStatusType(status, statusType);
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: could not set PhalaWorld StatusType[${statusType}]`
    ).to.be.true;
}

// Add Metadata for Spirit NFTs
async function addSpiritMetadata(khalaApi, overlord, metadata) {
    const tx = khalaApi.tx.pwNftSale.setSpiritsMetadata(metadata);
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: could not set Spirit metadata[${metadata}]`
    ).to.be.true;
}

// Start Claiming Spirits
async function usersClaimSpirits(khalaApi, recipients) {
    for (const recipient of recipients) {
        const index = recipients.indexOf(recipient);
        console.log(`\t[${index}]: Claiming Spirit for ${recipient.address}`);
        let result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.claimSpirit(), recipient);
        expect(
            result,
            `Error: account[${recipient.address}] failed to claim spirit`
        ).to.be.true;
    }
}

// Add Origin of Shells Metadata
async function addOriginOfShellsMetadata(khalaApi, overlord, originOfShellsMetadataArr) {
    const tx = khalaApi.tx.pwNftSale.setOriginOfShellsMetadata(originOfShellsMetadataArr);
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: could not set Origin of Shells metadata[${originOfShellsMetadataArr}]`
    ).to.be.true;
}

// Simulate Rare Origin of Shells Purchases
async function usersPurchaseRareOriginOfShells(khalaApi, recipientsInfo) {
    for (const recipient of recipientsInfo) {
        const account = recipient.account;
        const rarity = recipient.rarity;
        const race = recipient.race;
        const career = recipient.career;
        const amount = recipient.amount;
        let result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.buyRareOriginOfShell(rarity, race, career), account);
        expect(
            result,
            `Error: Failed Purchasing Rare Origin of Shell for owner: ${account.address}, rarity: ${rarity}, race: ${race}, career: ${career}, amount: ${amount}`
        ).to.be.true;
    }
}

// Users Purchase Whitelist Origin of Shells
async function usersPurchaseWhitelistOriginOfShells(khalaApi, recipientsInfo) {
    for (const recipient of recipientsInfo) {
        const account = recipient.account;
        const whitelistMessage = recipient.whitelistMessage;
        const race = recipient.race;
        const career = recipient.career;
        let result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.buyPrimeOriginOfShell(whitelistMessage, race, career), account);
        expect(
            result,
            `Error: Purchasing Prime Origin of Shell for owner: ${account.address}, whitelistMessage: ${whitelistMessage}, race: ${race}, career: ${career}`
        ).to.be.true
    }
}

// Update RarityType Count for Prime Origin Of Shells
async function updateRarityTypeCounts(khalaApi, overlord, rarityType, forSaleCount, giveawayCount) {
    let tx = khalaApi.tx.pwNftSale.updateRarityTypeCounts(rarityType, forSaleCount, giveawayCount);
    let result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: Could not update RarityType ${rarityType} inventory counts`
    ).to.be.true
}


// Users Preorder Origin of Shells
async function userPreorderOriginOfShell(khalaApi, preordersInfo) {
    for (const preorder of preordersInfo) {
        const account = preorder.account;
        const race = preorder.race;
        const career = preorder.career;
        const result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.preorderOriginOfShell(race, career), account);
        expect(
            result,
            `Error: Starting Preorder Origin of Shell account: ${account}, race: ${race}, career: ${career}...`
        ).to.be.true;
    }
}

// Mint chosen preorders
async function mintChosenPreorders(khalaApi, overlord, chosenPreorders) {
    let result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.mintChosenPreorders(chosenPreorders), overlord);
    expect(
        result,
        `Error: Mint Chosen Preorders ${chosenPreorders} Failed`
    ).to.be.true;
}

// Refund not chosen preorders
async function refundNotChosenPreorders(khalaApi, overlord, notChosenPreorders) {
    let result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwNftSale.refundNotChosenPreorders(notChosenPreorders), overlord);
    expect(
        result,
        `Error: Refunding Not Chosen Preorders ${notChosenPreorders} Failed`
    ).to.be.true;
}

// Set Incubation Process Status
async function setIncubationProcessStatus(khalaApi, overlord, status) {
    const tx = khalaApi.tx.pwIncubation.setCanStartIncubationStatus(status);
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: could not set PhalaWorld StatusType[${status}]`
    ).to.be.true;
}

async function initializeAccountsIncubationProcess(khalaApi, addresses) {
    for (const accountId of addresses) {
        const originOfShellCollectionId = await getOriginOfShellCollectionId(khalaApi);
        let nonceOwner = await getNonce(khalaApi, accountId.address);
        expect(
            originOfShellCollectionId.isSome,
            `Error: Origin of Shell Collection ID Not Set`
        ).to.be.true;
        let nfts = await getOwnedOriginOfShells(khalaApi, accountId, originOfShellCollectionId.unwrap().toNumber());

        for (const nft of nfts) {
            console.log(`\t${accountId.address} starting incubation for NFT ID: ${nft}...`);
            await khalaApi.tx.pwIncubation.startIncubation(originOfShellCollectionId.unwrap(), nft).signAndSend(accountId, {nonce: nonceOwner++});
        }
        await waitTxAccepted(khalaApi, accountId.address, nonceOwner - 1);
    }
}

// Simulate feeding Origin of Shells from array of accounts with which NFT they want to feed
async function simulateFeeding(khalaApi, accountFeedSimulation) {
    const originOfShellCollectionId = await getOriginOfShellCollectionId(khalaApi);
    expect(
        originOfShellCollectionId.isSome,
        `Error: Origin of Shell Collection ID Not Set`
    ).to.be.true;
    for (const accountFeedInfo of accountFeedSimulation) {
        const account = accountFeedInfo.account;
        const nftId = accountFeedInfo.feedTo;
        console.log(`\t${account.address} feeding [${originOfShellCollectionId.unwrap()}, ${nftId}]`);
        const result = await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwIncubation.feedOriginOfShell(originOfShellCollectionId.unwrap(), nftId), account);
        expect(
            result,
            `Error: could not feed [${originOfShellCollectionId.unwrap()}, ${nftId}]`
        ).to.be.true;
    }
}

// Set the ChosenPart for an account
async function setOriginOfShellChosenParts(khalaApi, overlord, collectionId, nftId, chosenParts) {
    const tx = khalaApi.tx.pwIncubation.setOriginOfShellChosenParts(collectionId, nftId, chosenParts);
    const result = await waitExtrinsicFinished(khalaApi, tx, overlord);
    expect(
        result,
        `Error: could not set Origin of Shell Chosen Parts [${chosenParts}]`
    ).to.be.true;
}

async function hatchOriginOfShell(khalaApi, overlord, originOfShellsOwners) {
    const defaultMetadata = "";
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    for (const accountId of originOfShellsOwners) {
        const originOfShellCollectionId = await getOriginOfShellCollectionId();
        expect(
            originOfShellCollectionId.isSome,
            `Error: Origin of Shell Collection ID Not Set`
        ).to.be.true;
        let nfts = await getOwnedOriginOfShells(khalaApi, accountId, originOfShellCollectionId.unwrap().toNumber());

        for (const nft of nfts) {
            console.log(`\t${accountId.address} hatching Origin of Shell for NFT ID: ${nft}...`);
            await khalaApi.tx.pwIncubation.hatchOriginOfShell(originOfShellCollectionId.unwrap(), nft, defaultMetadata).signAndSend(overlord, {nonce: nonceOverlord++});
        }
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
    }
}

module.exports = {
    pwCreateCollection,
    setStatusType,
    transferPha,
    setOverlordAccount,
    initializePhalaWorldClock,
    initializeRarityTypeCounts,
    setStatusType,
    addSpiritMetadata,
    usersClaimSpirits,
    addOriginOfShellsMetadata,
    usersPurchaseRareOriginOfShells,
    usersPurchaseWhitelistOriginOfShells,
    updateRarityTypeCounts,
    userPreorderOriginOfShell,
    mintChosenPreorders,
    refundNotChosenPreorders,
    setIncubationProcessStatus,
    initializeAccountsIncubationProcess,
    simulateFeeding,
    setOriginOfShellChosenParts,
    hatchOriginOfShell
}
