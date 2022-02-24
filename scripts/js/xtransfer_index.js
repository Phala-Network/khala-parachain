const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { encodeAddress, decodeAddress } = require("@polkadot/util-crypto");
const { MagicNumber } = require('@polkadot/types/metadata/MagicNumber');
const BN = require('bn.js');
const { gql, GraphQLClient } = require('graphql-request');

const bn1e12 = new BN(10).pow(new BN(12));

const khalaEndpoint = 'ws://127.0.0.1:9944'
const karuraEndpoint = 'ws://127.0.0.1:9955'

const khalaQueryEndpoint = 'https://api.subquery.network/sq/tolak/kahla-subquery-dev__dG9sY';
const karuraQueryEndpoint = 'https://api.subquery.network/sq/tolak/karura-subquery-dev__dG9sY';
const rinkebyQueryEndpoint = 'https://api.thegraph.com/subgraphs/name/tolak/khala-rinkeby-chainbridge';

const rinkebyGqlClient = new GraphQLClient(rinkebyQueryEndpoint, { timeout: 300000 });
const khalaGqlClient = new GraphQLClient(khalaQueryEndpoint, { timeout: 300000 });
const karuraGqlClient = new GraphQLClient(karuraQueryEndpoint, { timeout: 300000 });

async function getEvmSendHistory(account) {
    let data;
    try {
        data = await rinkebyGqlClient.request(gql`
        {
            bridgeOutboundingRecords (orderBy: createdAt, orderDirection: desc, filter: {sender: {equalTo: \"${account}\"}}) {
                id
                createdAt
                destChainId
                depositNonce
                resourceId
                amount
                recipient
                sendTx {
                    hash
                }
                sender
            }
        }
        `);
    } catch (e) {
        throw new Error(
          "Error getting bridgeOutboundingRecords from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
    return data;
}

async function getEvmReceiveHistory(originChainId, depositNonce) {
    let data;
    try {
        data = await rinkebyGqlClient.request(gql`
        {
            bridgeInboundingRecords (orderBy: createdAt, orderDirection: desc, filter: {originChainId: {equalTo: \"${originChainId}\"}, depositNonce: {equalTo: \"${depositNonce}\"}}) {
                id
                createdAt
                originChainId
                depositNonce
                resourceId
                status
                executeTx {
                    hash
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
          "Error getting bridgeOutboundingRecords from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
    return data;
}

async function getKhalaSendHistory(sender) {
    let data;
    try {
        data = await khalaGqlClient.request(gql`
        {
            bridgeOutboundingRecords (orderBy: CREATED_AT_DESC, filter: {sender: {equalTo: \"${sender}\"}}) {
                nodes {
                    id
                    createdAt
                    destChainId
                    depositNonce
                    resourceId
                    amount
                    recipient
                    sendTx
                    sender
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
        "Error getting revenue from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
  
    return data;
}

async function getKhalaSendHistoryByRecipient(recipient) {
    let data;
    try {
        data = await khalaGqlClient.request(gql`
        {
            bridgeOutboundingRecords (orderBy: CREATED_AT_DESC, filter: {recipient: {equalTo: \"${recipient}\"}}) {
                nodes {
                    id
                    createdAt
                    destChainId
                    depositNonce
                    resourceId
                    amount
                    recipient
                    sendTx
                    sender
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
        "Error getting revenue from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
  
    return data;
}

async function getKhalaReceiveHistory(originChainId, depositNonce) {
    let data;
    try {
        data = await khalaGqlClient.request(gql`
        {
            bridgeInboundingRecords (filter: {originChainId: {equalTo: \"${originChainId}\"}, depositNonce: {equalTo: \"${depositNonce}\"}}) {
                nodes {
                    id
                    createdAt
                    originChainId
                    depositNonce
                    resourceId
                    status
                    executeTx
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
        "Error getting revenue from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
  
    return data;
}

async function getKaruraSendHistory(sender) {
    let data;
    try {
        data = await karuraGqlClient.request(gql`
        {
            xTokenSents (orderBy: CREATED_AT_DESC, filter: {sender: {equalTo: \"${sender}\"}"}}) {
                nodes {
                    id
                    createdAt
                    sender
                    hash
                    destChain
                    recipient
                    amount
                    currencyId
                    isX3
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
        "Error getting revenue from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
  
    return data;
}

async function getKaruraReceiveHistory(recipient) {
    let data;
    try {
        data = await karuraGqlClient.request(gql`
        {
            currencyDeposits (filter: {recipient: {equalTo: \"${recipient}\"}"}}) {
                nodes {
                    id
                    createdAt
                    currencyId
                    recipient
                    amount
                }
            }
        }
        `);
    } catch (e) {
        throw new Error(
        "Error getting revenue from blockchain: " +
            JSON.stringify(e) +
            JSON.stringify(data)
        );
    }
  
    return data;
}

function evmHistoryFilter(api, evmHistory) {
    let x2History = [];
    let x3History = [];
    evmHistory.bridgeOutboundingRecords.filter(history => {
        try {
            let dest = api.createType('XcmV1MultiLocation', history.recipient).toJSON();
            if (dest.parents === 0 && dest.interior.hasOwnProperty('x1') && dest.interior.x1.hasOwnProperty('accountId32')) {   // to khala
                history.recipient = encodeAddress(dest.interior.x1.accountId32.id, 42).toString();
                x2History.push(history);
            } else if (dest.parents === 1 && dest.interior.hasOwnProperty('x1') && dest.interior.x1.hasOwnProperty('accountId32')) { // to relaychain
                history.recipient = encodeAddress(dest.interior.x1.accountId32.id, 42).toString();
                x3History.push(history);
            } else if (dest.parents === 1 && dest.interior.hasOwnProperty('x2') && dest.interior.x2[0].hasOwnProperty('parachain') && dest.interior.x2[1].hasOwnProperty('accountId32')) {  // to other parachians
                history.recipient = encodeAddress(dest.interior.x2[1].accountId32.id, 42).toString();
                history.destPara = dest.interior.x2[0].parachain;
                x3History.push(history);
            } else {
                throw new Error('EDEST');
            }
        } catch (e) {
            // Old recipient was not encoded from MultiLocation, it was just an khala account
            if (e.message !== 'EDEST') {
                x2History.push(history);
            } else {
                throw e;
            }
        }
    });

    return {
        x2History: x2History,
        x3History: x3History
    };
}

function karuraHistoryFilter(api, karuraSendHistory) {
    let x2History = [];
    let x3History = [];
    karuraSendHistory.nodes.filter(history => {
        if (history.isX3) {
            x3History.push(history);
        } else {
            x2History.push(history);
        }
    });

    return {
        x2History: x2History,
        x3History: x3History
    };
}

async function main() {
        const khalaProvider = new WsProvider(khalaEndpoint);
        const khalaApi = await ApiPromise.create({
            provider: khalaProvider,
        });
    
        const karuraProvider = new WsProvider(karuraEndpoint);
        const karuraApi = await ApiPromise.create({
            provider: karuraProvider,
        });

        const keyring = new Keyring({ type: 'sr25519' });
        const substrateAccount = keyring.addFromUri('//Alice');

        const rinkebyAccount = '0xA29D4E0F035cb50C0d78c8CeBb56Ca292616Ab20';
        // let privateKey = process.env.KEY;
        // let provider = new ethers.providers.JsonRpcProvider('https://rinkeby.infura.io/v3/6d61e7957c1c489ea8141e947447405b');
        // let ethereumWallet = new ethers.Wallet(privateKey, provider);
        // let bridge = new ethers.Contract(bridgeAddressOnRinkeby, BridgeJson.abi, ethereumWallet);

        // Get EVM account send history
        let evmSendHistory = await getEvmSendHistory(rinkebyAccount);
        // Filter x2 and x3 history
        let filterSendResult = evmHistoryFilter(khalaApi, evmSendHistory);
    
        // x2 query history(Here we just pick the latest one to demonstrate)
        let evmX2QueryHistory = filterSendResult.x2History[0];
        let evmX2Confirmation = await getKhalaReceiveHistory(evmX2QueryHistory.destChainId, evmX2QueryHistory.depositNonce);
        console.log(`===> X2(from EVM) confirm results on khala:\n${JSON.stringify(evmX2Confirmation, null, 4)}`);

        // x3 query history(Here we just pick the latest one to demonstrate)
        let evmX3QueryHistory = filterSendResult.x3History[0];
        let evmX3ForwardInfo = await getKhalaReceiveHistory(evmX3QueryHistory.destChainId, evmX3QueryHistory.depositNonce);
        console.log(`===> X3(from EVM) forward results on khala:\n${JSON.stringify(evmX3ForwardInfo, null, 4)}`);
        let evmX3Confirmation = await getKaruraReceiveHistory(evmX3QueryHistory.recipient);
        console.log(`===> X3(from EVM) confirmation on khala:\n${JSON.stringify(evmX3Confirmation, null, 4)}`);

        /*********************** Following logic is transfer that send to from other parachians to EVM ******************** */
        // Get substrate account send history(x3)
        let karuraSendHistory = await getKaruraSendHistory(substrateAccount.address);
        console.log(`===> Send history from karura:\n${JSON.stringify(karuraSendHistory, null, 4)}`);

        // For x2(e.g. send from karura to khala through xcm), field `isX3` should be false,
        // then check khala xcmDeposit records
        let karuraX2SendHistory = karuraSendHistory.x2History[0];

        // For x3(e.g. send from karura to EVM through xcm and bridge), field `isX3` should be true,
        // get forward info by BridgeOutboundingRecord on khala and get confirmation result by
        // BridgeInboudingRecord in EVM chains
        let karuraX3SendHistory = karuraSendHistory.x3History[0];
        // Use `recipient` to associate with BridgeOutboundingRecord in khala
        // Note: same `recipient` may received more than one asset, so we don't know which one is acctually
        // associate the origin send transaction, so just use latest to demonstrate the example
        let karuraX3ForwardInfo = (await getKhalaSendHistoryByRecipient(karuraX3SendHistory.recipient))[0];
        console.log(`===> X3(from karura) forward results on khala:\n${JSON.stringify(karuraX2ForwardInfo, null, 4)}`);
        let karuraX3Confirmation = await getEvmReceiveHistory(karuraX3ForwardInfo.destChainId, karuraX3ForwardInfo.depositNonce);
        console.log(`===> X3(from karura) confirmation on EVM:\n${JSON.stringify(karuraX3Confirmation, null, 4)}`);
}

main().catch(console.error).finally(() => process.exit());
