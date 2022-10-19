const { getApiConnection, getAccount, alicePrivkey, evePrivkey, ferdiePrivkey, overlordPrivkey, bobPrivkey,
    charliePrivkey, davidPrivkey
} = require('../khala/khalaApi');
const { setIncubationProcessStatus, initializeAccountsIncubationProcess, simulateFeeding, mintChosenPreorders, setOriginOfShellChosenParts
} = require('../util/tx');
const { getTopNFed } = require('../util/helpers');
const { getEra } = require('../util/fetch');

describe("Start Origin of Shell Incubation Process", () => {
    let api;
    let alice, bob, charlie, david, eve, ferdie, overlord;

    before(async () => {
        api = await getApiConnection();
    });
    it(`Enable Incubation Process`, async () => {
        overlord = await getAccount(overlordPrivkey);
        await setIncubationProcessStatus(api, overlord, true);
    });
    it(`Initialize Accounts Incubation Process`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
        const addresses = [alice, bob, charlie, david, eve, ferdie];
        await initializeAccountsIncubationProcess(api, addresses);
    });
    it(`Simulate Feeding between Accounts`, async () => {
        alice = await getAccount(alicePrivkey);
        bob = await getAccount(bobPrivkey);
        charlie = await getAccount(charliePrivkey);
        david = await getAccount(davidPrivkey);
        eve = await getAccount(evePrivkey);
        ferdie = await getAccount(ferdiePrivkey);
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
    });
    it(`Get Top 6 Origin of Shells Fed`, async () => {
        const currentEra = await getEra(api);
        const top6Fed = await getTopNFed(api, currentEra, 6);
        let i = 0;
        for (const originOfShellCollectionIdNftId in top6Fed) {
            console.log(`\t[${i}]: ${originOfShellCollectionIdNftId}`);
            i++;
        }
    });
    it(`Set Origin of Shell Chosen Parts`, async () => {
        overlord = await getAccount(overlordPrivkey);
        // TODO: Generate parts helper function
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
    });

    after(() => {
        api.disconnect();
    });
});