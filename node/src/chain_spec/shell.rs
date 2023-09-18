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
use sc_service::ChainType;
use crate::chain_spec::{
    Extensions, get_account_id_from_seed
};
use serde::Deserialize;
use sp_core::sr25519;

use shell_parachain_runtime::AccountId;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
    sc_service::GenericChainSpec<shell_parachain_runtime::RuntimeGenesisConfig, Extensions>;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GenesisInfo {
    root_key: AccountId,
}

pub fn development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Shell Development Testnet",
        "shell_dev_testnet",
        ChainType::Local,
        move || genesis(
            id, get_account_id_from_seed::<sr25519::Public>("Alice")
        ),
        Vec::new(),
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "westend".into(),
            para_id: id.into(),
            runtime: "shell".to_string(),
        },
    )
}

pub fn staging_config() -> ChainSpec {
    let genesis_info_bytes = include_bytes!("../../res/shell_genesis_info.json");
    let genesis_info: GenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Shell",
        "shell",
        ChainType::Live,
        move || genesis(2264u32.into(), genesis_info.root_key.clone()),
        Vec::new(),
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "kusama".into(),
            para_id: 2264u32.into(),
            runtime: "shell".to_string(),
        },
    )
}

fn genesis(
    parachain_id: ParaId,
    root_key: AccountId,
) -> shell_parachain_runtime::RuntimeGenesisConfig {
    shell_parachain_runtime::RuntimeGenesisConfig {
        system: shell_parachain_runtime::SystemConfig {
            code: shell_parachain_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            ..Default::default()
        },
        parachain_info: shell_parachain_runtime::ParachainInfoConfig {
            parachain_id,
            ..Default::default()
        },
        parachain_system: Default::default(),
        sudo: shell_parachain_runtime::SudoConfig { key: Some(root_key) },
        polkadot_xcm: shell_parachain_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(3),
            ..Default::default()
        },
        balances: shell_parachain_runtime::BalancesConfig {
            balances: vec![],
        },
    }
}
