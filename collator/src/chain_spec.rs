use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use khala_runtime::{AccountId, AuraId, Signature};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup, Properties};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
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
        testnet_properties(),
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn khala_local_config(id: ParaId) -> ChainSpec {
    // stash, controller, session-key
    // generated with secret:
    // for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
    // and
    // for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done
    let initial_authorities: Vec<(AccountId, AuraId)> = vec![
        (
            // 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
            hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
            // 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
            hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"]
                .unchecked_into(),
        ),
        (
            // 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
            hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
            // 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
            hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"]
                .unchecked_into(),
        ),
        (
            // 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
            hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
            // 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
            hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"]
                .unchecked_into(),
        ),
        (
            // 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
            hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
            // 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
            hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"]
                .unchecked_into(),
        ),
    ];

    // generated with secret: subkey inspect "$secret"/fir
    let root_key: AccountId = hex![
        // 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
        "ce4f5981f1b33b5371454c753ac6f3a703e57e1fe20c7b26a1f9bda024008221"
    ]
    .into();

    let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

    ChainSpec::from_genesis(
        "Khala Testnet",
        "khala_testnet",
        ChainType::Live,
        move || {
            khala_testnet_genesis(
                root_key.clone(),
                initial_authorities.clone(), // TODO:
                endowed_accounts.clone(),
                id,
            )
        },
        Vec::new(),
        None,
        Some("khala"),
        testnet_properties(),
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

fn khala_testnet_genesis(
    root_key: AccountId,
    initial_authorities: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
) -> khala_runtime::GenesisConfig {
    khala_runtime::GenesisConfig {
        frame_system: khala_runtime::SystemConfig {
            code: khala_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_balances: khala_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        },
        pallet_sudo: khala_runtime::SudoConfig { key: root_key },
        cumulus_pallet_parachain_info: khala_runtime::ParachainInfoConfig { parachain_id: id },
        pallet_aura: khala_runtime::AuraConfig {
            authorities: initial_authorities
                .iter()
                .cloned()
                .map(|(_, aura)| aura)
                .collect(),
        },
        cumulus_pallet_aura_ext: Default::default(),
        pallet_collective_Instance1: khala_runtime::CouncilConfig::default(),
        pallet_collective_Instance2: khala_runtime::TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((endowed_accounts.len() + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },
        pallet_membership_Instance1: Default::default(),
        pallet_treasury: Default::default(),
        pallet_vesting: Default::default(),
        pallet_democracy: khala_runtime::DemocracyConfig::default(),
    }
}

fn testnet_properties() -> Option<Properties> {
    let mut p = Properties::new();

    p.insert("tokenSymbol".into(), "PHA".into());
    p.insert("tokenDecimals".into(), 12.into());
    p.insert("ss58Format".into(), 30.into());

    Some(p)
}
