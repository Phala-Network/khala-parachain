const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const { getCollectionsCount } = require('./fetch');
const { token, waitExtrinsicFinished, getNonce, getCollectionType } = require('./helpers');

chai.use(chaiAsPromised);
const expect = chai.expect;

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
    if (expectCollectionType === -1) {
        console.log(`Error: invalid collectionType[${collectionType}]`);
        expect(expectCollectionType).to.be.not.equal(-1, `Error: invalid collectionType[${collectionType}]`);
    } else if (expectCollectionType.isSome) {
        expect(
            expectCollectionType.isSome,
    `Error: collectionType[${collectionType}] is already set to collectionId[${expectCollectionType.unwrap().toNumber()}]`
        ).to.be.false;
    }
    const tx = khalaApi.tx.pwNftSale.pwCreateCollection(metadata, maxOptional, symbol);
    // TODO: Handle Events if we want to expand functionality later
    await waitExtrinsicFinished(khalaApi, tx, overlord);
    const newCollectionCount = await getCollectionsCount(khalaApi);
    expect(newCollectionCount.toNumber()).to.be.equal(
        oldCollectionCount.toNumber() + 1,
        "Error: NFT collection count should increase"
    );
    await pwSetCollection(khalaApi, overlord, oldCollectionCount.toNumber(), collectionType);
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
        return new Error(`Error: Incorrect collectionType[${collectionType}]`);
    }
    await waitExtrinsicFinished(khalaApi, tx, overlord);
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

// Set StatusType for
async function setStatusType(khalaApi, overlord, statusType, status) {
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    return new Promise(async (resolve) => {
        console.log(`Setting ${statusType} to ${status}...`);
        const unsub = await khalaApi.tx.pwNftSale.setStatusType(status, statusType
        ).signAndSend(overlord, {nonce: nonceOverlord++}, ({ events, status }, result) => {
            if (status.isInBlock) {
                console.log(`Transaction included at blockHash ${status.asInBlock}`);
            } else if (status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
                unsub();
                resolve();
            }
        });
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
    });
}

module.exports = {
    pwCreateCollection,
    setStatusType
}