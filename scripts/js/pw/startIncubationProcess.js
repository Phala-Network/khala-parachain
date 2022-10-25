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

async function setIncubationProcess(khalaApi, overlord, status) {
    let nonceOverlord = await getNonce(khalaApi, overlord.address);
    return new Promise(async (resolve) => {
        console.log(`Setting CanStartIncubationStatus to ${status}...`);
        const unsub = await khalaApi.tx.pwIncubation.setCanStartIncubationStatus(status
        ).signAndSend(overlord, {nonce: nonceOverlord++}, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
                unsub();
                resolve();
            }
        });
        await waitTxAccepted(khalaApi, overlord.address, nonceOverlord - 1);
        console.log(`Enabling the Incubation Process...Done`);
    });
}

// Start incubation process for all accounts
async function initializeAccountsIncubationProcess(khalaApi, addresses) {
    for (const accountId of addresses) {
        const originOfShellCollectionId = await khalaApi.query.pwNftSale.originOfShellCollectionId();
        let nonceOwner = await getNonce(khalaApi, accountId.address);
        let nfts = [];
        if (originOfShellCollectionId.isSome) {
            const originOfShells = await khalaApi.query.uniques.account.entries(accountId.address, originOfShellCollectionId.unwrap());
            originOfShells
                .map(([key, _value]) =>
                    [key.args[0].toString(), key.args[1].toNumber(), key.args[2].toNumber()]
                ).forEach(([acct, collectionId, nftId]) => {
                nfts.push(nftId);
                console.log({
                    acct,
                    collectionId,
                    nftId,
                })
            })
        } else {
            throw new Error(
                'Origin of Shell Collection ID not configured'
            )
        }
        for (const nft of nfts) {
            console.log(`${accountId.address} starting incubation for NFT ID: ${nft}...`);
            await khalaApi.tx.pwIncubation.startIncubation(originOfShellCollectionId.unwrap(), nft).signAndSend(accountId, { nonce: nonceOwner++});
            console.log(`${accountId.address} starting incubation for NFT ID: ${nft}...Done`);
        }
        await waitTxAccepted(khalaApi, accountId.address, nonceOwner - 1);
    }
}

// Simulate feeding Origin of Shells from array of accounts with which NFT they want to feed
async function simulateFeeding(khalaApi, accountFeedSimulation) {
    console.log(`Starting Origin of Shell Feeding Simulation...`);
    const originOfShellCollectionId = await khalaApi.query.pwNftSale.originOfShellCollectionId();
    if (originOfShellCollectionId.isSome) {
        for (const accountFeedInfo of accountFeedSimulation) {
            const account = accountFeedInfo.account;
            const nftId = accountFeedInfo.feedTo;
            console.log(`${account.address} feeding [${originOfShellCollectionId.unwrap()}, ${nftId}]`);
            await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwIncubation.feedOriginOfShell(originOfShellCollectionId.unwrap(), nftId), account);
        }
    }
    console.log(`Origin of Shell Feeding Simulation...Done`);
}

// Set the ChosenPart for an account
async function setOriginOfShellChosenParts(khalaApi, root, collectionId, nftId, chosenParts) {
    console.log(`Setting Origin of Shell Parts...`);
    await waitExtrinsicFinished(khalaApi, khalaApi.tx.pwIncubation.setOriginOfShellChosenParts(collectionId, nftId, chosenParts), root);
    console.log(`Setting Origin of Shell Parts...DONE`);
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
    // Use Overlord account to enable the incubation phase
    await setIncubationProcess(api, overlord, true);
    // Accounts with Origin of Shells
    const addresses = [alice, bob, charlie, david, eve, ferdie];
    // Initialize incubation process
    await initializeAccountsIncubationProcess(api, addresses);
    // Start Feeding simulation
    const accountFeedSimulation = [
        {'account': alice, 'feedTo': 0},
        {'account': bob, 'feedTo': 0},
        {'account': charlie, 'feedTo': 1},
        {'account': david, 'feedTo': 2},
        {'account': eve, 'feedTo': 3},
        {'account': ferdie, 'feedTo': 4},
        {'account': alice, 'feedTo': 5},
        {'account': bob, 'feedTo': 1},
        {'account': charlie, 'feedTo': 3},
        {'account': david, 'feedTo': 0},
        {'account': eve, 'feedTo': 1},
        {'account': ferdie, 'feedTo': 4},
    ];
    await simulateFeeding(api, accountFeedSimulation);
    const currentEra = await api.query.pwNftSale.era();
    console.log(`Current Era: ${currentEra}`);
    // Times fed in era 0 for the [collectionId, nftId], era
    const originOfShellFoodStats = await api.query.pwIncubation.originOfShellFoodStats.entries(currentEra.toNumber());

    const sortedOriginOfShellStats = originOfShellFoodStats
        .map(([key, value]) => {
                const eraId = key.args[0].toNumber()
                const collectionIdNftId = key.args[1].toHuman()
                const numTimesFed = value.toNumber()
                return {
                    eraId: eraId,
                    collectionIdNftId: collectionIdNftId,
                    numTimesFed: numTimesFed
                }
            }
        ).sort((a, b) => b.numTimesFed - a.numTimesFed);
    console.log(sortedOriginOfShellStats.slice(0,10));
    let reduceHatchTimeSeconds = [10800, 7200, 3600, 2400, 1400, 1400, 1400, 1400, 1400, 1400]
    let topTenFed = [];
    let i = 0;
    for (const nftStats in sortedOriginOfShellStats) {
        topTenFed[i] = api.createType('((u32, u32), u64)', [sortedOriginOfShellStats[nftStats].collectionIdNftId, reduceHatchTimeSeconds[i]]);
        i++
    }
    console.log(topTenFed.toString());

    const CyborgChosenParts = {
        "parts": {
            "weapon":  {
                "shell_part": {
                    "name": "Weapon",
                    "metadata": null,
                    "layer": 0,
                    'x': 0,
                    'y': 0,
                },
                "sub_parts": [
                    {
                        "name": "lightsaber",
                        "rarity": "Normal",
                        "career": "RoboWarrior",
                        "style": "Sg01",
                        "layer": 19,
                        "metadata": "pw://weapon/lighsaber",
                        "tradeable": true,
                    },
                    {
                        "name": "Sniper Rifle",
                        "rarity": "Rare",
                        "career": "RoboWarrior",
                        "style": "Sp01",
                        "layer": 20,
                        "metadata": "pw://weapon/sniper-rifle",
                        "tradeable": true,
                    },
                ]
            },
            "body": {
                "shell_part": {
                    "name": "Body",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA"],
                    "layer": 0,
                    'x': 0,
                    'y': 0,
                },
                "sub_parts": [
                    {
                        "name": "Short Hair",
                        "style": "Sg01",
                        "rarity": "Normal",
                        "metadata": "pw://body/hair/short-hair",
                        "layer": 2,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Human Eyes",
                        "rarity": "Normal",
                        "metadata": "pw://body/eyes/human-eyes",
                        "layer": 4,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Male Head 01",
                        "style": "Skin01",
                        "rarity": "Normal",
                        "metadata": "pw://body/head/male-head-01",
                        "layer": 6,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Male Body 01",
                        "style": "Skin01",
                        "rarity": "Normal",
                        "metadata": "pw://body/body/male-body-01",
                        "layer": 16,
                        'x': 0,
                        'y': 0,
                    },
                ],
            },
            "jaw": {
                "shell_part": {
                    "name": "Cyborg Jaw(Neck)",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA"],
                    "style": "Me01",
                    "metadata": "pw://jaw",
                    "layer": 3,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
            },
            "tattoo": {
                "shell_part": {
                    "name": "Cyberpsychosis Tatoo",
                    "rarity": "Epic",
                    "race": "Cyborg",
                    "sizes": ["MA"],
                    "style": "Sp01",
                    "metadata": "pw://tattoo",
                    "layer": 5,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
            },
            "jacket": {
                "shell_part": {
                    "name": "Jacket",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "layer": 0,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
                "sub_parts": [
                    {
                        "name": "White Buckle",
                        "rarity": "Normal",
                        "style": "Sg01",
                        "metadata": "pw://jacket/jacket_details",
                        "layer": 7,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Solid Jacket",
                        "rarity": "Normal",
                        "style": "Sg01",
                        "metadata": "pw://jacket/jacket",
                        "layer": 8,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Lightspeed Hood",
                        "rarity": "Legend",
                        "style": "Sp01",
                        "metadata": "pw://jacket/hood",
                        "layer": 17,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Solid Jacket",
                        "rarity": "Normal",
                        "style": "Sg01",
                        "metadata": "pw://jacket/jacketun",
                        "layer": 18,
                        'x': 0,
                        'y': 0,
                    },
                ],
            },
            "bionic_arm": {
                "shell_part": {
                    "name": "Black Bionic Arm",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "style": "Sg01",
                    "metadata": "pw://bionic_arm",
                    "layer": 9,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
            },
            "shirt": {
                "shell_part": {
                    "name": "Cyborg's symbol T-Shirt",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "style": "Sg07",
                    "metadata": "pw://shirt",
                    "layer": 10,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
            },
            "shorts": {
                "shell_part": {
                    "name": "Shorts",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "metadata": null,
                    "layer": 0,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
                "sub_parts": [
                    {
                        "name": "Black Buckle",
                        "rarity": "Normal",
                        "style": "Sg01",
                        "metadata": "pw://shorts/shorts-details",
                        "layer": 11,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Solid Shorts",
                        "rarity": "Normal",
                        "style": "Sg01",
                        "metadata": "pw://shorts/shorts",
                        "layer": 12,
                        'x': 0,
                        'y': 0,
                    },
                ],
            },
            "feet": {
                "shell_part": {
                    "name": "Feet",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "layer": 0,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
                "sub_parts": [
                    {
                        "name": "Solid Shorts Buckle",
                        "rarity": "Normal",
                        "style": "Sg04",
                        "metadata": "pw://feet/shoes-ribbon",
                        "layer": 13,
                        'x': 0,
                        'y': 0,
                    },
                    {
                        "name": "Black Fade Shorts",
                        "rarity": "Epic",
                        "style": "Gc02",
                        "metadata": "pw://feet/shoes",
                        "layer": 14,
                        'x': 0,
                        'y': 0,
                    }
                ]
            },
            "bionic_leg": {
                "shell_part": {
                    "name": "Black Bionic Leg",
                    "rarity": "Normal",
                    "race": "Cyborg",
                    "sizes": ["MA", "MB"],
                    "style": "Sg01",
                    "metadata": "pw://bionic_leg",
                    "layer": 15,
                    'x': 0,
                    'y': 0,
                    "tradeable": true,
                },
            },
        },
    };
    // Set chosen part for NFT ID 0
    await setOriginOfShellChosenParts(api, overlord, 1, 0, CyborgChosenParts);

}

main().catch(console.error).finally(() => process.exit());