const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const BN = require('bn.js');

const bn1e12 = new BN(10).pow(new BN(12));
const bn1e8 = new BN(10).pow(new BN(8));
const PHA_PER_SECOND = bn1e12.mul(new BN(80));
const ASSETS = ['PHA', 'KSM', 'KAR', 'BNC', 'VSKSM', 'ZLK'];
const FEEE_PRICES = [
    PHA_PER_SECOND,
    PHA_PER_SECOND.div(new BN(600)),
    PHA_PER_SECOND.div(new BN(8)),
    PHA_PER_SECOND.div(new BN(4)),
    PHA_PER_SECOND.div(new BN(600)),
    PHA_PER_SECOND.div(new BN(4))
];
const SENDING_XCM_FEE_IN_PHA = bn1e8.mul(new BN(512));
const RECEIVING_XCM_FEE_IN_PHA = bn1e8.mul(new BN(640));

async function getBridgeSendToSoloChainFeeInPha(khalaApi, destChain, amount) {
    let result = await khalaApi.query.bridgeTransfer.bridgeFee(destChain);
    let minFee = new BN(result[0]);
    let feeScale = new BN(result[1]);
    let feeExtimated = amount.mul(feeScale).div(new BN(1000));
    return feeExtimated.gte(minFee) ? feeExtimated : minFee;
}

async function getBridgeSendToSoloChainFee(khalaApi, destChain, asset, amount) {
    let feeExtimatedInPha = await getBridgeSendToSoloChainFeeInPha(khalaApi, destChain, amount);
    if (asset === 'PHA') {
        return feeExtimatedInPha;
    } else {
        let index = ASSETS.findIndex(item => item === asset);
        if (index === -1) throw new Error('Unrecognized asset');
        return feeExtimatedInPha.mul(FEEE_PRICES[index]).div(PHA_PER_SECOND);
    }
}

// Return fee deducted when sending assets to khala from parachians
// Note: fee only affected by asset type
function getXcmSendFromParachainsFee(asset) {
    if (asset === 'PHA') {
        return RECEIVING_XCM_FEE_IN_PHA;
    } else {
        let index = ASSETS.findIndex(item => item === asset);
        if (index === -1) throw new Error('Unrecognized asset');
        return RECEIVING_XCM_FEE_IN_PHA.mul(FEEE_PRICES[index]).div(PHA_PER_SECOND)
    }
}

// Return fee deducted when sending assets from khala to parachains
// Note: fee only affected by asset type
function getXcmSendFromKhalaFee(asset) {
    if (asset === 'PHA') {
        return SENDING_XCM_FEE_IN_PHA;
    } else {
        let index = ASSETS.findIndex(item => item === asset);
        if (index === -1) throw new Error('Unrecognized asset');
        return SENDING_XCM_FEE_IN_PHA.mul(FEEE_PRICES[index]).div(PHA_PER_SECOND)
    }
}

async function main() {
    const khalaProvider = new WsProvider('ws://localhost:9944');
    const khalaApi = await ApiPromise.create({
        provider: khalaProvider,
    });

    // We use karura as an example of parachain, other parachains like bifrost have
    // the same logic

    let bridgeFeeOfPHA = await getBridgeSendToSoloChainFee(khalaApi, 0, 'PHA', bn1e12.mul(new BN(100)));
    console.log(`(X2)Fee of transfer PHA from khala to ethereum: ${bridgeFeeOfPHA}`);
    let bridgeFeeOfKAR = await getBridgeSendToSoloChainFee(khalaApi, 0, 'KAR', bn1e12.mul(new BN(100)))
    console.log(`(X2)Fee of transfer KAR from khala to ethereum: ${bridgeFeeOfKAR}`);

    console.log(`(X2)Fee of transfer PHA from ethereum to khala: ${0}`);
    console.log(`(X2)Fee of transfer KAR from ethereum to khala: ${0}`);

    let xcmSendFeeOfPHA = getXcmSendFromKhalaFee('PHA');
    console.log(`(X2)Fee of transfer PHA from khala to karura: ${xcmSendFeeOfPHA}`);
    let xcmSendFeeOfKAR = getXcmSendFromKhalaFee('KAR');
    console.log(`(X2)Fee of transfer KAR from khala to karura: ${xcmSendFeeOfKAR}`);

    let xcmReceiveFeeOfPHA = getXcmSendFromParachainsFee('PHA');
    console.log(`(X2)Fee of transfer PHA from karura to khala: ${xcmReceiveFeeOfPHA}`);
    let xcmReceiveFeeOfKAR = getXcmSendFromParachainsFee('KAR');
    console.log(`(X2)Fee of transfer KAR from karura to khala: ${xcmReceiveFeeOfKAR}`);

    // X3 send from solochain to parachains has the same fee with transfer from khala
    // to other parachains
    console.log(`(X3)Fee of transfer PHA from ethereum to karura: ${xcmSendFeeOfPHA}`);
    console.log(`(X3)Fee of transfer KAR from ethereum to karura: ${xcmSendFeeOfKAR}`);

    // The most expensive fee should be X3 transfer that send from parachains to solo chains,
    // total fee consists of xcm transfer fee and bridge transfer fee
    console.log(`(X3)Fee of transfer PHA from karura to ethereum: ${xcmReceiveFeeOfPHA.add(bridgeFeeOfPHA)}`);
    console.log(`(X3)Fee of transfer KAR from karura to ethereum: ${xcmReceiveFeeOfKAR.add(bridgeFeeOfKAR)}`);
}

main().catch(console.error).finally(() => process.exit());
