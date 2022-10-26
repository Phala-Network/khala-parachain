require('dotenv').config();
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const payeePrivkey = process.env.PAYEE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

async function getAccount(accountUri) {
    const keyring = new Keyring({type: 'sr25519'});
    return keyring.addFromUri(accountUri);
}

async function getApiConnection() {
    const wsProvider = new WsProvider(endpoint);
    return await ApiPromise.create({
        provider: wsProvider,
        types: {
            Purpose: {
                _enum: [
                    'RedeemSpirit',
                    'BuyPrimeOriginOfShells',
                ],
            },
            OverlordMessage: {
                account: 'AccountId',
                purpose: 'Purpose',
            }
        }
    });
}

module.exports = {
    getApiConnection,
    getAccount,
    alicePrivkey,
    bobPrivkey,
    charliePrivkey,
    davidPrivkey,
    evePrivkey,
    ferdiePrivkey,
    overlordPrivkey,
    payeePrivkey
}