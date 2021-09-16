#!/usr/bin/env node

const yargs = require('yargs/yargs')
const { hideBin } = require('yargs/helpers')

const argv = yargs(hideBin(process.argv))
    .option('endpoint')
    .demandOption(['endpoint'])
    .argv

require('dotenv').config();

const { HttpProvider } = require('@polkadot/rpc-provider/http');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const typedefs = require('@phala/typedefs').khalaDev;

(async() => {
    const endpoint = argv["endpoint"]

    let provider = null;
    if (endpoint.startsWith('wss://') || endpoint.startsWith('ws://')) {
        provider = new WsProvider(endpoint)
    } else if (endpoint.startsWith('https://') || endpoint.startsWith('http://')) {
        provider = new HttpProvider(endpoint)
    } else {
        console.warn(`Invalid endpoint ${endpoint}`)
        process.exit(1)
    }

    const api = await ApiPromise.create({
        provider: provider,
        types: typedefs
    })

    const onChainTimestamp = (await api.query.timestamp.now()).toNumber();
    const localTimestamp = Date.now();
    const gapInMinutes = (localTimestamp - onChainTimestamp) / 1000 / 60;

    const headBlockNumber = (await api.rpc.chain.getHeader()).number.toNumber();

    console.log({
        onChainTimestamp,
        localTimestamp,
        gapInMinutes,
        headBlockNumber
    })
})().catch(console.error).finally(() => process.exit());
