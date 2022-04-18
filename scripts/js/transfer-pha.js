require('dotenv').config();

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const BN = require('bn.js');
const ethers = require('ethers');
const BridgeJson = require('./Bridge.json');

const bn1e18 = new BN(10).pow(new BN(18));
const bridgeAddressOnRinkeby = '0x0712Cf53B9fA1A33018d180a4AbcC7f1803F55f4';
const bridgeAddressOnMainnet = '0x8F92e7353b180937895E0C5937d616E8ea1A2Bb9';
const phaResourceId = '0x00e6dfb61a2fb903df487c401663825643bb825d41695e63df8af6162ab145a6';

// EVM => Khala, call Bridge.Deposit()
async function transferPhaFromEvmToKhala(khalaApi, bridge, recipient, amount) {
    const khalaChainId = 1;
    // dest is not Account public key any more.
    const dest = khalaApi.createType('XcmV1MultiLocation', {
        // parents = 0 means we send to xcm local network(e.g. Khala network here)
        parents: 0,
        interior: khalaApi.createType('Junctions', {
            X1: khalaApi.createType('XcmV1Junction', {
                AccountId32: {
                    network: khalaApi.createType('XcmV0JunctionNetworkId', 'Any'),
                    id: recipient,
                }
            })
        })
    }).toHex();

    const data = '0x' +
        ethers.utils.hexZeroPad(ethers.BigNumber.from(amount.toString()).toHexString(), 32).substr(2) +
        ethers.utils.hexZeroPad(ethers.utils.hexlify((dest.length - 2) / 2), 32).substr(2) +
        dest.substr(2);

    const tx = await bridge.deposit(khalaChainId, phaResourceId, data);
    console.log(`Transfer PHA from EVM to Khala: ${tx.hash}`);

}


async function main() {
    const recipient = process.env.RECIPIENT;
    const amount = Number(process.env.AMOUNT);
    const privateKey = process.env.KEY;

    const khalaProvider = new WsProvider('wss://khala.api.onfinality.io/public-ws');
    const khalaApi = await ApiPromise.create({
        provider: khalaProvider,
    });

    const provider = new ethers.providers.JsonRpcProvider('https://mainnet.infura.io/v3/6d61e7957c1c489ea8141e947447405b');
    const ethereumWallet = new ethers.Wallet(privateKey, provider);
    const bridge = new ethers.Contract(bridgeAddressOnMainnet, BridgeJson.abi, ethereumWallet);

    await transferPhaFromEvmToKhala(khalaApi, bridge, recipient, bn1e18.mul(new BN(amount)));
}

main().catch(console.error).finally(() => process.exit());
