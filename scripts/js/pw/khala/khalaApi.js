require('dotenv').config();
const { ApiPromise, WsProvider } = require('@polkadot/api');

const endpoint = process.env.ENDPOINT;

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