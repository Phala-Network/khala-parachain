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
    Extensions,
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
    sc_service::GenericChainSpec<shell_parachain_runtime::GenesisConfig, Extensions>;

pub fn local_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Shell Local Testnet",
        "shell_local_testnet",
        ChainType::Local,
        move || testnet_genesis(id),
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

fn testnet_genesis(parachain_id: ParaId) -> shell_parachain_runtime::GenesisConfig {
    shell_parachain_runtime::GenesisConfig {
        system: shell_parachain_runtime::SystemConfig {
            code: shell_parachain_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_info: shell_parachain_runtime::ParachainInfoConfig { parachain_id },
        parachain_system: Default::default(),
    }
}
