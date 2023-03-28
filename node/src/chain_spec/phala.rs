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
use phala_parachain_runtime::{AccountId, AuraId};
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
    sc_service::GenericChainSpec<phala_parachain_runtime::GenesisConfig, Extensions>;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn session_keys(keys: AuraId) -> phala_parachain_runtime::opaque::SessionKeys {
    phala_parachain_runtime::opaque::SessionKeys { aura: keys }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GenesisInfo {
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<(AccountId, String)>,
    technical_committee: Vec<AccountId>,
}

pub fn development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Phala Local Testnet",
        "phala_local_testnet",
        ChainType::Local,
        move || {
            genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                ],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                ],
                vec![
                    (get_account_id_from_seed::<sr25519::Public>("Alice"), 1 << 60),
                ],
                id,
            )
        },
        vec![],
        None,
        Some("phala"),
        None,
        chain_properties(),
        Extensions {
            relay_chain: "polkadot-dev".into(),
            para_id: id.into(),
            runtime: "phala".to_string(),
        },
    )
}

pub fn local_config(id: ParaId) -> ChainSpec {
    // Master key:
    // extend split brush maximum nominee oblige merit modify latin never shiver slide
    //
    // - Root: <master>/phala
    // - Collator account: <master>//validator//<idx>
    // - Collator session key: <master>//validator//<idx>//aura
    //
    // Learn more: scripts/js/genPhalaGenesis.js
    let genesis_info_bytes = include_bytes!("../../res/phala_local_genesis_info.json");
    local_testnet_config(id, genesis_info_bytes, "polkadot-local")
}

fn local_testnet_config(id: ParaId, genesis_info_bytes: &[u8], relay_chain: &str) -> ChainSpec {
    let genesis_info: GenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Phala Testnet",
        "phala_testnet",
        ChainType::Live,
        move || {
            use std::str::FromStr;
            let genesis_info = genesis_info.clone();
            genesis(
                genesis_info.root_key,
                genesis_info.initial_authorities,
                genesis_info.technical_committee,
                genesis_info
                    .endowed_accounts
                    .into_iter()
                    .map(|(k, amount)| (k, u128::from_str(&amount).expect("Bad amount; qed.")))
                    .collect(),
                id,
            )
        },
        Vec::new(),
        None,
        Some("phala"),
        None,
        chain_properties(),
        Extensions {
            relay_chain: relay_chain.into(),
            para_id: id.into(),
            runtime: "phala".to_string(),
        },
    )
}

pub fn staging_config() -> ChainSpec {
    let genesis_info_bytes = include_bytes!("../../res/phala_genesis_info.json");
    let genesis_info: GenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Phala",
        "phala",
        ChainType::Live,
        move || {
            use std::str::FromStr;
            let genesis_info = genesis_info.clone();
            genesis(
                genesis_info.root_key,
                genesis_info.initial_authorities,
                genesis_info.technical_committee,
                genesis_info
                    .endowed_accounts
                    .into_iter()
                    .map(|(k, amount)| (k, u128::from_str(&amount).expect("Bad amount; qed.")))
                    .collect(),
                2035u32.into(),
            )
        },
        Vec::new(),
        None,
        Some("phala"),
        None,
        chain_properties(),
        Extensions {
            relay_chain: "polkadot".into(),
            para_id: 2035,
            runtime: "phala".to_string(),
        },
    )
}

fn genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    technical_committee: Vec<AccountId>,
    endowed_accounts: Vec<(AccountId, u128)>,
    id: ParaId,
) -> phala_parachain_runtime::GenesisConfig {
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

    phala_parachain_runtime::GenesisConfig {
        system: phala_parachain_runtime::SystemConfig {
            code: phala_parachain_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        balances: phala_parachain_runtime::BalancesConfig {
            balances: endowed_accounts,
        },
        sudo: phala_parachain_runtime::SudoConfig { key: Some(root_key) },
        parachain_info: phala_parachain_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: phala_parachain_runtime::CollatorSelectionConfig {
            invulnerables: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, _)| acc)
                .collect(),
            candidacy_bond: phala_parachain_runtime::constants::currency::UNIT * 16, // 16 PHA
            ..Default::default()
        },
        session: phala_parachain_runtime::SessionConfig {
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
        council: phala_parachain_runtime::CouncilConfig {
            members: vec![],
            phantom: Default::default(),
        },
        technical_committee: phala_parachain_runtime::TechnicalCommitteeConfig {
            members: technical_committee,
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        treasury: Default::default(),
        vesting: phala_parachain_runtime::VestingConfig { vesting: vec![] },
        democracy: Default::default(),
        phragmen_election: Default::default(),
        polkadot_xcm: phala_parachain_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
    }
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
