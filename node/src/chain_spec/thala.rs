// Copyright (C) 2021 HashForest Technology Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use thala_parachain_runtime::{AccountId, AuraId};
use sc_chain_spec::Properties;
use sc_service::ChainType;
use serde::Deserialize;
use sp_core::sr25519;
use crate::chain_spec::{
    get_collator_keys_from_seed, get_account_id_from_seed,
    Extensions,
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
    sc_service::GenericChainSpec<thala_parachain_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn session_keys(keys: AuraId) -> thala_parachain_runtime::opaque::SessionKeys {
    thala_parachain_runtime::opaque::SessionKeys { aura: keys }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GenesisInfo {
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<(AccountId, String)>,
    #[allow(dead_code)]
    technical_committee: Vec<AccountId>,
}

pub fn development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Khala Local Testnet",
        "khala_local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                id,
                Some(dev_registry_config(get_account_id_from_seed::<
                    sr25519::Public,
                >("Alice"))),
            )
        },
        vec![],
        None,
        Some("khala"),
        chain_properties(),
        Extensions {
            relay_chain: "kusama-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn local_config(id: ParaId) -> ChainSpec {
    // Master key:
    // extend split brush maximum nominee oblige merit modify latin never shiver slide
    //
    // - Root: <master>/khala
    // - Collator account: <master>//validator//<idx>
    // - Collator session key: <master>//validator//<idx>//aura
    //
    // Learn more: scripts/js/genKhalaGenesis.js
    let genesis_info_bytes = include_bytes!("../../res/khala_local_genesis_info.json");
    local_testnet_config(id, genesis_info_bytes, "kusama-local")
}

fn local_testnet_config(id: ParaId, genesis_info_bytes: &[u8], relay_chain: &str) -> ChainSpec {
    let genesis_info: GenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Khala Testnet",
        "khala_testnet",
        ChainType::Live,
        move || {
            let genesis_info = genesis_info.clone();
            testnet_genesis(
                genesis_info.root_key,
                genesis_info.initial_authorities,
                genesis_info
                    .endowed_accounts
                    .into_iter()
                    .map(|(k, _)| k)
                    .collect(),
                id,
                None,
            )
        },
        Vec::new(),
        None,
        Some("khala"),
        chain_properties(),
        Extensions {
            relay_chain: relay_chain.into(),
            para_id: id.into(),
        },
    )
}

fn genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    technical_committee: Vec<AccountId>,
    endowed_accounts: Vec<(AccountId, u128)>,
    id: ParaId,
    dev_registry_override: Option<thala_parachain_runtime::PhalaRegistryConfig>,
) -> thala_parachain_runtime::GenesisConfig {
    let all_accounts: Vec<_> = initial_authorities
        .iter()
        .map(|(k, _)| k)
        .chain(&technical_committee)
        .chain(&[root_key.clone()])
        .cloned()
        .collect();
    if !check_accounts_endowed(&all_accounts, &endowed_accounts) {
        panic!("All the genesis accounts must be endowed; qed.")
    }

    thala_parachain_runtime::GenesisConfig {
        system: thala_parachain_runtime::SystemConfig {
            code: thala_parachain_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: thala_parachain_runtime::BalancesConfig {
            balances: endowed_accounts,
        },
        sudo: thala_parachain_runtime::SudoConfig { key: root_key },
        parachain_info: thala_parachain_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: thala_parachain_runtime::CollatorSelectionConfig {
            invulnerables: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, _)| acc)
                .collect(),
            candidacy_bond: thala_parachain_runtime::constants::currency::UNIT * 16, // 16 PHA
            ..Default::default()
        },
        session: thala_parachain_runtime::SessionConfig {
            keys: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),              // account id
                        acc.clone(),              // validator id
                        session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        council: thala_parachain_runtime::CouncilConfig {
            members: vec![],
            phantom: Default::default(),
        },
        technical_committee: thala_parachain_runtime::TechnicalCommitteeConfig {
            members: technical_committee,
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        treasury: Default::default(),
        vesting: thala_parachain_runtime::VestingConfig { vesting: vec![] },
        democracy: Default::default(),
        phragmen_election: Default::default(),
        phala_registry: dev_registry_override.unwrap_or(
            thala_parachain_runtime::PhalaRegistryConfig {
                workers: Vec::new(),
                gatekeepers: Vec::new(),
                benchmark_duration: 50,
            },
        ),
        phala_mining: Default::default(),
    }
}

fn testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    dev_registry_override: Option<thala_parachain_runtime::PhalaRegistryConfig>,
) -> thala_parachain_runtime::GenesisConfig {
    // Testnet setup:
    // - 1,152,921 PHA per endowed account
    // - 1/2 endowed accounts are listed in technical committee
    let endowment: Vec<_> = endowed_accounts
        .iter()
        .cloned()
        .map(|acc| (acc, 1 << 60))
        .collect();
    let technical_committee: Vec<_> = endowed_accounts
        .iter()
        .take((endowed_accounts.len() + 1) / 2)
        .cloned()
        .collect();
    genesis(
        root_key,
        initial_authorities,
        technical_committee,
        endowment,
        id,
        dev_registry_override,
    )
}

fn chain_properties() -> Option<Properties> {
    let mut p = Properties::new();

    p.insert("tokenSymbol".into(), "PHA".into());
    p.insert("tokenDecimals".into(), 12.into());
    p.insert("ss58Format".into(), 30.into());

    Some(p)
}

/// Checks all the given accounts are endowed
fn check_accounts_endowed(
    accounts: &Vec<AccountId>,
    endowed_accounts: &Vec<(AccountId, u128)>,
) -> bool {
    accounts.iter().all(|account| {
        endowed_accounts
            .iter()
            .any(|(endowed, _)| account == endowed)
    })
}

fn dev_registry_config(operator: AccountId) -> thala_parachain_runtime::PhalaRegistryConfig {
    // The pubkey of "0x1"
    let raw_dev_sr25519_pubkey: [u8; 32] =
        hex!["3a3d45dc55b57bf542f4c6ff41af080ec675317f4ed50ae1d2713bf9f892692d"];
    let dev_sr25519_pubkey = sp_core::sr25519::Public::from_raw(raw_dev_sr25519_pubkey);
    let dev_ecdh_pubkey =
        hex!["3a3d45dc55b57bf542f4c6ff41af080ec675317f4ed50ae1d2713bf9f892692d"].to_vec();

    thala_parachain_runtime::PhalaRegistryConfig {
        workers: vec![(
            dev_sr25519_pubkey.clone(),
            dev_ecdh_pubkey,
            Some(operator.clone()),
        )],
        gatekeepers: vec![dev_sr25519_pubkey],
        benchmark_duration: 1,
    }
}
