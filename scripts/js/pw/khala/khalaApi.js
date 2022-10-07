import {Keyring} from "@polkadot/api";

require('dotenv').config();
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

const alicePrivkey = process.env.ROOT_PRIVKEY;
const bobPrivkey = process.env.USER_PRIVKEY;
const overlordPrivkey = process.env.OVERLORD_PRIVKEY;
const ferdiePrivkey = process.env.FERDIE_PRIVKEY;
const charliePrivkey = process.env.CHARLIE_PRIVKEY;
const davidPrivkey = process.env.DAVID_PRIVKEY;
const evePrivkey = process.env.EVE_PRIVKEY;
const endpoint = process.env.ENDPOINT;

const keyring = new Keyring({type: 'sr25519'});

export const alice = keyring.addFromUri(alicePrivkey);
export const bob = keyring.addFromUri(bobPrivkey);
export const charlie = keyring.addFromUri(charliePrivkey);
export const david = keyring.addFromUri(davidPrivkey);
export const eve = keyring.addFromUri(evePrivkey);
export const ferdie = keyring.addFromUri(ferdiePrivkey);
export const overlord = keyring.addFromUri(overlordPrivkey);

export function customAccount(accountUri) {
    keyring.addFromUri(accountUri)
}

export async function getApiConnection() {
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