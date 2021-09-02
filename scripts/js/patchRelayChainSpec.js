// USAGE: node patchRelayChainSpec.js ../../tmp/specs/test-rococo.json > ../../tmp/specs/test-rococo-patched.json

const fs = require('fs');

const { Keyring } = require('@polkadot/api');
const { cryptoWaitReady, encodeAddress } = require('@polkadot/util-crypto');

function keystore() {
    const sr = new Keyring({ type: 'sr25519' });
    const ed = new Keyring({ type: 'ed25519' });
    const ec = new Keyring({ type: 'ecdsa', prefix: 43 });
    function getSr(suri) {
        return sr.createFromUri(suri).address;
    }
    function getEd(suri) {
        return ed.createFromUri(suri).address;
    }
    function getEc(suri) {
        // Walk around https://github.com/polkadot-js/common/issues/1099
        const raw = ec.createFromUri(suri, undefined, 'ecdsa').publicKey;
        return encodeAddress(raw, 43);
    }
    return {
        get(n) {
            return [
                getSr(`//${n}//stash`),
                getSr(`//${n}`),
                {
                    grandpa: getEd(`//${n}`),
                    babe: getSr(`//${n}`),
                    im_online: getSr(`//${n}`),
                    para_validator: getSr(`//${n}`),
                    para_assignment: getSr(`//${n}`),
                    authority_discovery: getSr(`//${n}`),
                    beefy: getEc(`//${n}`),
                }
            ]
        }
    }
}

function hackJsonBigInt(json) {
    return json.replace(/"BigInt\((\d+)\)"/g, '$1');
}


async function main() {
    const pathIn = process.argv[2];
    const raw = fs.readFileSync(pathIn, {encoding: 'utf-8'});
    const spec = JSON.parse(raw);
    const keys = keystore();
    const players = ['Alice', 'Bob', 'Charlie', 'Dave'];

    await cryptoWaitReady();

    // change balances
    spec.genesis.runtime.runtime_genesis_config.balances.balances = players
        .flatMap(n => keys.get(n).slice(0, 2))
        .map(addr => [addr, 'BigInt(1000000000000000000)']);
    // change sudo
    spec.genesis.runtime.runtime_genesis_config.sudo.key = keys.get('Alice')[1];
    // change session length (2mins)
    spec.genesis.runtime.session_length_in_blocks = 20;
    // inject initial keys
    spec.genesis.runtime.runtime_genesis_config.session.keys = players.map(n => keys.get(n));
    // override parachain configs
    spec.genesis.runtime.runtime_genesis_config.configuration.config = {
        ...spec.genesis.runtime.runtime_genesis_config.configuration.config,
        ...parachainsConfigurationOverride,
    };
    // misc
    spec.bootNodes = ["/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD"];
    spec.telemetryEndpoints = [];

    const json = JSON.stringify(spec, undefined, 2);
    console.log(hackJsonBigInt(json));
}

const parachainsConfigurationOverride = {
    // "max_code_size": 5242880,
    "max_head_data_size": 20480,
    "max_upward_queue_count": 10,
    "max_upward_queue_size": 51200,
    "max_upward_message_size": 51200,
    "max_upward_message_num_per_candidate": 10,
    "hrmp_max_message_num_per_candidate": 10,
    "validation_upgrade_frequency": 1,
    "validation_upgrade_delay": 1,
    "max_pov_size": 5242880,
    "max_downward_message_size": 51200,
    "preferred_dispatchable_upward_messages_step_weight": 100000000000,
    "hrmp_max_parachain_outbound_channels": 10,
    "hrmp_max_parathread_outbound_channels": 0,
    "hrmp_open_request_ttl": 2,
    "hrmp_sender_deposit": 1009100000000000,
    "hrmp_recipient_deposit": 1009100000000000,
    "hrmp_channel_max_capacity": 1000,
    "hrmp_channel_max_total_size": 102400,
    "hrmp_max_parachain_inbound_channels": 10,
    "hrmp_max_parathread_inbound_channels": 0,
    "hrmp_channel_max_message_size": 102400,
    "code_retention_period": 28800,
    "parathread_cores": 0,
    "parathread_retries": 0,
    "group_rotation_frequency": 20,
    "chain_availability_period": 4,
    "thread_availability_period": 4,
    "scheduling_lookahead": 1,
    "max_validators_per_core": 5,
    "max_validators": 200,
    "dispute_period": 6,
    "dispute_post_conclusion_acceptance_period": 600,
    "dispute_max_spam_slots": 2,
    "dispute_conclusion_by_time_out_period": 600,
    "no_show_slots": 2,
    "n_delay_tranches": 40,
    "zeroth_delay_tranche_width": 0,
    "needed_approvals": 2,
    "relay_vrf_modulo_samples": 0
};

main().catch(console.error).finally(() => process.exit());
