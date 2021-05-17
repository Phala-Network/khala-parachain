// USAGE
// NUM_VALIDATORS=3 NUM_TECH_COMMITTEE=0 ENDOWMENT='' MNEMONIC='xxx' OUT=../../node/res/khala_local_genesis_info.json node gen_khala_genesis.js

require('dotenv').config();

const { Keyring } = require('@polkadot/keyring');
const { cryptoWaitReady, mnemonicGenerate } = require('@polkadot/util-crypto');
const fs = require('fs');

const NUM_VALIDATORS = parseInt(process.env.NUM_VALIDATORS) || 3;
const NUM_TECH_COMMITTEE = parseInt(process.env.NUM_TECH_COMMITTEE ?? 1);
const ENDOWMENT = process.env.ENDOWMENT ?? '1000' + '000000000000';  // 1000 PHA

async function main() {
    await cryptoWaitReady();

    const keyring = new Keyring({ type: 'sr25519', ss58Format: 30 });
    const mnemonic = process.env.MNEMONIC || mnemonicGenerate();
    console.warn('mnemonic:', mnemonic);

    const rootKey = keyring.addFromUri(`${mnemonic}/khala`).address;
    const initialAuthorities = [];
    for (let i = 0; i < NUM_VALIDATORS; i++) {
        const validator = keyring.addFromUri(`${mnemonic}//validator//${i}`);
        const aura = keyring.addFromUri(`${mnemonic}//validator//${i}//aura`);
        initialAuthorities.push([validator.address, aura.address]);
    }
    const technicalCommittee = [];
    for (let i = 0; i < NUM_TECH_COMMITTEE; i++) {
        const member = keyring.addFromUri(`${mnemonic}//techcomm//${i}`);
        technicalCommittee.push(member.address);
    }
    const endowedAccounts = initialAuthorities
        .map(([v]) => v)
        .concat(technicalCommittee)
        .concat([rootKey])
        .map(k => [k, ENDOWMENT]);

    const genesis = {
      rootKey,
      initialAuthorities,
      endowedAccounts,
      technicalCommittee,
    };
    console.log(genesis);

    if (process.env.OUT) {
        const jsonGenesisInfo = JSON.stringify(genesis, undefined, 2);
        fs.writeFileSync(process.env.OUT, jsonGenesisInfo + '\n', {encoding: 'utf-8'});
    }
}

main().catch(console.error).finally(() => {process.exit()});