use cumulus_primitives_core::ParaId;
use khala_runtime::{AccountId, AuraId, Signature};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup, Properties};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<khala_runtime::GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_pair_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_pair_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_pair_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn khala_session_keys(keys: AuraId) -> khala_runtime::opaque::SessionKeys {
    khala_runtime::opaque::SessionKeys { aura: keys }
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct KhalaGenesisInfo {
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<(AccountId, String)>,
    technical_committee: Vec<AccountId>,
}

type AccountPublic = <Signature as Verify>::Signer;

pub fn khala_development_config(id: ParaId) -> ChainSpec {
    ChainSpec::from_genesis(
        "Khala Local Testnet",
        "khala_local_testnet",
        ChainType::Local,
        move || {
            khala_testnet_genesis(
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
            )
        },
        vec![],
        None,
        Some("khala"),
        chain_properties(),
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn khala_local_config(id: ParaId) -> ChainSpec {
    let genesis_info_bytes = include_bytes!("../res/khala_local_genesis_info.json");
    let genesis_info: KhalaGenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Khala Testnet",
        "khala_testnet",
        ChainType::Live,
        move || {
            let genesis_info = genesis_info.clone();
            khala_testnet_genesis(
                genesis_info.root_key,
                genesis_info.initial_authorities,
                genesis_info
                    .endowed_accounts
                    .into_iter()
                    .map(|(k, _)| k)
                    .collect(),
                id,
            )
        },
        Vec::new(),
        None,
        Some("khala"),
        chain_properties(),
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn khala_staging_config() -> ChainSpec {
    let genesis_info_bytes = include_bytes!("../res/khala_genesis_info.json");
    let genesis_info: KhalaGenesisInfo =
        serde_json::from_slice(genesis_info_bytes).expect("Bad genesis info; qed.");

    ChainSpec::from_genesis(
        "Khala",
        "khala",
        ChainType::Live,
        move || {
            use std::str::FromStr;
            let genesis_info = genesis_info.clone();
            khala_genesis(
                genesis_info.root_key,
                genesis_info.initial_authorities,
                genesis_info.technical_committee,
                genesis_info
                    .endowed_accounts
                    .into_iter()
                    .map(|(k, amount)| (k, u128::from_str(&amount).expect("Bad amount; qed.")))
                    .collect(),
                2004.into(),
            )
        },
        Vec::new(),
        None,
        Some("khala"),
        chain_properties(),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: 2004,
        },
    )
}

fn khala_genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    technical_committee: Vec<AccountId>,
    endowed_accounts: Vec<(AccountId, u128)>,
    id: ParaId,
) -> khala_runtime::GenesisConfig {
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

    khala_runtime::GenesisConfig {
        frame_system: khala_runtime::SystemConfig {
            code: khala_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_balances: khala_runtime::BalancesConfig {
            balances: endowed_accounts,
        },
        pallet_sudo: khala_runtime::SudoConfig { key: root_key },
        cumulus_pallet_parachain_info: khala_runtime::ParachainInfoConfig { parachain_id: id },
        pallet_collator_selection: khala_runtime::CollatorSelectionConfig {
            invulnerables: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, _)| acc)
                .collect(),
            candidacy_bond: khala_runtime::EXISTENTIAL_DEPOSIT * 160, // 16 PHA
            ..Default::default()
        },
        pallet_session: khala_runtime::SessionConfig {
            keys: initial_authorities
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),              // account id
                        acc.clone(),              // validator id
                        khala_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        pallet_aura: Default::default(),
        cumulus_pallet_aura_ext: Default::default(),
        pallet_collective_Instance1: khala_runtime::CouncilConfig::default(),
        pallet_collective_Instance2: khala_runtime::TechnicalCommitteeConfig {
            members: technical_committee,
            phantom: Default::default(),
        },
        pallet_membership_Instance1: Default::default(),
        pallet_treasury: Default::default(),
        pallet_vesting: Default::default(),
        pallet_democracy: khala_runtime::DemocracyConfig::default(),
    }
}

fn khala_testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
) -> khala_runtime::GenesisConfig {
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
    khala_genesis(
        root_key,
        initial_authorities,
        technical_committee,
        endowment,
        id,
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
