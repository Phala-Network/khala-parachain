require('dotenv').config();
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { token, waitTxAccepted, getNonce } = require('./pwUtils');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

// Transfer balance to an account
async function transferPha(khalaApi, sender, recipients, amount) {
    let senderNonce = await getNonce(khalaApi, sender.address);
    return new Promise(async (resolve) => {
        console.log(`Starting transfers...`);
        for (const recipient of recipients) {
            const index = recipients.indexOf(recipient);
            console.log(`[${index}]: Transferring... ${amount.toString()} PHA from ${sender.address} to ${recipient.address}`);
            const unsub = await khalaApi.tx.balances.transfer(recipient.address, amount).signAndSend(sender, {nonce: senderNonce++}, (result) => {
                if (result.status.isInBlock) {
                    console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
                } else if (result.status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                    unsub();
                    resolve();
                }
            });
            console.log(`[${index}]: Transferring...DONE`);
        }
        await waitTxAccepted(khalaApi, sender.address, senderNonce - 1);
    });
}

// Set Overlord Account with Sudo account
async function setOverlordAccount(khalaApi, sender, newOverlord) {
    let senderNonce = await getNonce(khalaApi, sender.address);
    return new Promise(async (resolve) => {
        console.log("Setting new overlord...");
        const unsub = await khalaApi.tx.sudo.sudo(
            khalaApi.tx.pwNftSale.setOverlord(newOverlord.address)
        ).signAndSend(sender, {nonce: senderNonce++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
        await waitTxAccepted(khalaApi, sender.address, senderNonce - 1);
    });
}

// Initialize Phala World Clock, create Spirits, Origin Shell & Shell Collections & set NFT inventory with Overlord account
async function initPhalaWorld(khalaApi, overlord) {
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    return new Promise(async (resolve) => {
        console.log("Initialize Phala World Clock...");
        const unsub = await khalaApi.tx.pwNftSale.initializeWorldClock()
            .signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
                if (result.status.isInBlock) {
                    console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
                } else if (result.status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                    unsub();
                    resolve();
                }
            });
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
        console.log("Initialize Phala World Clock...Done");
        console.log("Create Spirits, Origin of Shells & Shells Collections...");
        // mint spirits NFTs with overlord
        // collection 0: spirits
        const unsub2 = await khalaApi.tx.pwNftSale.pwCreateCollection(
            'Phala World Spirits Collection',
            null,
            'PWSPRT'
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub2();
                resolve();
            }
        });
        // set the spirits collection id
        const unsub3 = await khalaApi.tx.pwNftSale.setSpiritCollectionId(
            0
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub3();
                resolve();
            }
        });
        // collection 1: origin of shells
        const unsub4 = await khalaApi.tx.pwNftSale.pwCreateCollection(
            'PhalaWorld Origin of Shells Collection',
            null,
            'PWOAS'
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub4();
                resolve();
            }
        });
        // set the origin of shell collection id
        const unsub5 = await khalaApi.tx.pwNftSale.setOriginOfShellCollectionId(
            1
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub5();
                resolve();
            }
        });
        console.log("Create Spirits and Origin of Shell Collections...Done");
        console.log("Create Shell and Shell Parts Collections...");
        // collection 2: shells
        const unsub6 = await khalaApi.tx.pwNftSale.pwCreateCollection(
            'PhalaWorld Shells Collection',
            null,
            'PWSHL'
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub6();
                resolve();
            }
        });
        // set the shell collection id
        const unsub7 = await khalaApi.tx.pwIncubation.setShellCollectionId(
            2
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub7();
                resolve();
            }
        });
        // collection 3: shell parts
        const unsub8 = await khalaApi.tx.pwNftSale.pwCreateCollection(
            'PhalaWorld Shell Parts Collection',
            null,
            'PWPRT'
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub8();
                resolve();
            }
        });
        // set the shell parts collection id
        const unsub9 = await khalaApi.tx.pwIncubation.setShellPartsCollectionId(
            3
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub9();
                resolve();
            }
        });
        console.log("Create Shell and Shell Parts Collections...Done");
        console.log("Initialize Origin of Shell NFT sale inventory...");
        // set the initial inventory numbers that will be used until the preorder phase
        const unsub10 = await khalaApi.tx.pwNftSale.initRarityTypeCounts()
            .signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
                if (result.status.isInBlock) {
                    console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
                } else if (result.status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                    unsub10();
                    resolve();
                }
            });
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
        console.log("Initialize Origin of Shell NFT sale inventory...Done");
    });
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
    const userAccounts = [overlord, bob, charlie, david, eve, ferdie];

    // Send PHA to Account from Alice
    await transferPha(api, alice, userAccounts, token(20_000));
    // Set Overlord account
    await setOverlordAccount(api, alice, overlord);

    // Initialize Phala World
    await initPhalaWorld(api, overlord);
}

main().catch(console.error).finally(() => process.exit());