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
    // it(`Enable Incubation Process`, async () => {
    //     overlord = await getAccount(overlordPrivkey);
    //     await setIncubationProcessStatus(api, overlord, true);
    // });
    // it(`Initialize Accounts Incubation Process`, async () => {
    //     alice = await getAccount(alicePrivkey);
    //     bob = await getAccount(bobPrivkey);
    //     charlie = await getAccount(charliePrivkey);
    //     david = await getAccount(davidPrivkey);
    //     eve = await getAccount(evePrivkey);
    //     ferdie = await getAccount(ferdiePrivkey);
    //     const addresses = [alice, bob, charlie, david, eve, ferdie];
    //     await initializeAccountsIncubationProcess(api, addresses);
    // });
    // it(`Simulate Feeding between Accounts`, async () => {
    //     alice = await getAccount(alicePrivkey);
    //     bob = await getAccount(bobPrivkey);
    //     charlie = await getAccount(charliePrivkey);
    //     david = await getAccount(davidPrivkey);
    //     eve = await getAccount(evePrivkey);
    //     ferdie = await getAccount(ferdiePrivkey);
    //     const accountFeedSimulation = [
    //         {'account': alice, 'feedTo': 0},
    //         {'account': bob, 'feedTo': 0},
    //         {'account': charlie, 'feedTo': 1},
    //         {'account': david, 'feedTo': 2},
    //         {'account': eve, 'feedTo': 3},
    //         {'account': ferdie, 'feedTo': 4},
    //         {'account': alice, 'feedTo': 5},
    //         {'account': bob, 'feedTo': 1},
    //         {'account': charlie, 'feedTo': 3},
    //         {'account': david, 'feedTo': 0},
    //         {'account': eve, 'feedTo': 1},
    //         {'account': ferdie, 'feedTo': 4},
    //     ];
    //     await simulateFeeding(api, accountFeedSimulation);
    // });
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
        const jacketChosenPart = {
            'parts': {
                "jacket": {
                    'shell_part': {
                        'name': "jacket",
                        'rarity': 'Magic',
                        'metadata': null,
                        'layer': 0,
                        'x': 0,
                        'y': 0,
                    },
                    'sub_parts': [
                        {
                            'name': "jacket-details",
                            'rarity': 'Legendary',
                            'metadata': "ar://jacket-details-uri",
                            'layer': 0,
                            'x': 0,
                            'y': 0
                        },
                        {
                            'name': "jacket-hat",
                            'rarity': 'Magic',
                            'metadata': "ar://jacket-hat-uri",
                            'layer': 0,
                            'x': 0,
                            'y': 0
                        },
                        {
                            'name': "jacket",
                            'rarity': 'Prime',
                            'metadata': "ar://jacket-uri",
                            'layer': 0,
                            'x': 0,
                            'y': 0
                        }
                    ],
                },
                't_shirt': {
                    'shell_part': {
                        'name': 't_shirt',
                        'rarity': 'Prime',
                        'metadata': "ar://t-shirt-uri",
                        'layer': 0,
                        'x': 0,
                        'y': 0,
                    },
                    'sub_parts': null,
                },
                'shoes': {
                    'shell_part': {
                        'name': "shoes",
                        'rarity': 'Prime',
                        'metadata': null,
                        'layer': 0,
                        'x': 0,
                        'y': 0,
                    },
                    'sub_parts': [
                        {
                            'name': "shoes-details",
                            'rarity': 'Prime',
                            'metadata': "ar://shoes-details-uri",
                            'layer': 0,
                            'x': 0,
                            'y': 0
                        },
                        {
                            'name': "shoes",
                            'rarity': 'Prime',
                            'metadata': "ar://shoes-uri",
                            'layer': 0,
                            'x': 0,
                            'y': 0
                        }
                    ],
                }
            }
        };
        // Set chosen part for NFT ID 0
        await setOriginOfShellChosenParts(api, overlord, 1, 0, jacketChosenPart);
    });

    after(() => {
        api.disconnect();
    });
});