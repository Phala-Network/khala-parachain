// Copyright (C) 2021 HashForest Technology Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Phala runtime.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
#![allow(clippy::identity_op)]

// Make the WASM binary available.
#[cfg(all(feature = "std", feature = "include-wasm"))]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is \
        built with `SKIP_WASM_BUILD` flag and it is only usable for \
        production chains. Please rebuild with the flag disabled.",
    )
}

// Defaults helpers used in chain spec
pub mod defaults;
mod weights;
// Constant values used within the runtime.
pub mod constants;
use constants::{
    currency::*,
    fee::{pha_per_second, WeightToFee},
};

mod migrations;
mod msg_routing;

use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, Block as BlockT, Bounded, ConvertInto,
        TrailingZeroInput,
    },
    transaction_validity::{TransactionSource, TransactionValidity},
    AccountId32, ApplyExtrinsicResult, DispatchError, FixedPointNumber, Perbill, Percent, Permill,
    Perquintill,
};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    match_types,
    pallet_prelude::Get,
    parameter_types,
    traits::{
        tokens::nonfungibles::*, AsEnsureOriginWithArg, ConstU32, Contains, Currency,
        EitherOfDiverse, EqualPrivilegeOnly, Everything, Imbalance, InstanceFilter, IsInVec,
        KeyOwnerProofSystem, LockIdentifier, Nothing, OnUnbalanced, Randomness, U128CurrencyToVote,
        WithdrawReasons, SortedMembers,
    },
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        ConstantMultiplier, IdentityFee, Weight,
    },
    BoundedVec, PalletId, RuntimeDebug, StorageValue,
};

use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned, EnsureSignedBy,
};

use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::{prelude::*, Weight as XCMWeight};
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds,
    FungiblesAdapter, ParentIsPreset, RelayChainAsNative, NoChecking, MintLocation,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor, traits::WithOriginFilter};

pub use subbridge_pallets::{
    chainbridge, dynamic_trader::DynamicWeightTrader, fungible_adapter::XTransferAdapter, helper,
    xcmbridge, xtransfer,
};
use sygma_traits::{DepositNonce, DomainID};

use pallet_rmrk_core::{CollectionInfoOf, InstanceInfoOf, PropertyInfoOf, ResourceInfoOf};
use pallet_rmrk_equip::{BaseInfoOf, BoundedThemeOf, PartTypeOf};
use rmrk_traits::{
    primitives::*,
    primitives::{CollectionId, NftId, ResourceId},
    NftChild,
};

pub use parachains_common::{rmrk_core, rmrk_equip, uniques, Index, *};

#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_timestamp::Call as TimestampCall;
pub use phala_pallets::{
    pallet_base_pool, pallet_computation, pallet_mq, pallet_phat, pallet_phat_tokenomic,
    pallet_registry, pallet_stake_pool, pallet_stake_pool_v2, pallet_vault,
    pallet_wrapped_balances,
};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;
    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, Hasher>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
        }
    }
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("phala"),
    impl_name: create_runtime_str!("phala"),
    authoring_version: 1,
    spec_version: 1243,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 5,
    state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

/// All migrations executed on runtime upgrade as a nested tuple of types implementing
/// `OnRuntimeUpgrade`.
type Migrations = ();

type EnsureRootOrHalfCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

construct_runtime! {
    pub struct Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        // System support stuff
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip::{Pallet, Storage} = 2,
        Utility: pallet_utility::{Pallet, Call, Event} = 3,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 4,
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 5,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 6,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 7,
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 8,

        // Parachain staff
        ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config} = 20,
        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned} = 21,

        // XCM helpers
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 31,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 32,
        PolkadotXcm: pallet_xcm::{Pallet, Storage, Call, Event<T>, Origin, Config} = 33,

        // Monetary stuff
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 40,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 41,
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 42,

        // Collator support. the order of these 5 are important and shall not change.
        Authorship: pallet_authorship::{Pallet, Storage} = 50,
        CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 51,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 52,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 53,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 54,

        // Governance
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 60,
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 61,
        Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 62,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 63,
        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 64,
        Lottery: pallet_lottery::{Pallet, Call, Storage, Event<T>} = 65,
        TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 66,
        TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 67,
        PhragmenElection: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 68,
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 69,
        ChildBounties: pallet_child_bounties::{Pallet, Call, Storage, Event<T>} = 70,

        // Main, starts from 80

        // Bridges
        ChainBridge: chainbridge::{Pallet, Call, Storage, Event<T>} = 80,
        XcmBridge: xcmbridge::{Pallet, Event<T>, Storage} = 81,
        XTransfer: xtransfer::{Pallet, Call, Storage, Event<T>} = 82,
        AssetsRegistry: assets_registry::{Pallet, Call, Storage, Event<T>} = 83,

        // Phala
        PhalaMq: pallet_mq::{Pallet, Call, Storage} = 85,
        PhalaRegistry: pallet_registry::{Pallet, Call, Event<T>, Storage, Config<T>} = 86,
        PhalaComputation: pallet_computation::{Pallet, Call, Event<T>, Storage, Config} = 87,
        PhalaStakePool: pallet_stake_pool::{Pallet, Event<T>, Storage} = 88,
        PhalaStakePoolv2: pallet_stake_pool_v2::{Pallet, Call, Event<T>, Storage} = 89,
        PhalaVault: pallet_vault::{Pallet, Call, Event<T>, Storage} = 90,
        PhalaWrappedBalances: pallet_wrapped_balances::{Pallet, Call, Event<T>, Storage} = 91,
        PhalaBasePool: pallet_base_pool::{Pallet, Call, Event<T>, Storage} = 92,
        PhalaPhatContracts: pallet_phat::{Pallet, Call, Event<T>, Storage} = 93,
        PhalaPhatTokenomic: pallet_phat_tokenomic::{Pallet, Call, Event<T>, Storage} = 94,
        // `sudo` has been removed on production
        // Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 255,

        // Phala World
        Uniques: pallet_uniques::{Pallet, Call, Storage, Event<T>} = 101,
        RmrkCore: pallet_rmrk_core::{Pallet, Call, Event<T>, Storage} = 102,
        RmrkEquip: pallet_rmrk_equip::{Pallet, Call, Event<T>, Storage} = 103,
        RmrkMarket: pallet_rmrk_market::{Pallet, Call, Storage, Event<T>} = 104,

        // 11x kept for Sygma bridge

        // inDEX
        PalletIndex: pallet_index::{Pallet, Call, Storage, Event<T>} = 121,
    }
}

pub struct BaseCallFilter;
impl Contains<RuntimeCall> for BaseCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        if let RuntimeCall::PolkadotXcm(xcm_method) = call {
            return match xcm_method {
                pallet_xcm::Call::force_xcm_version { .. }
                | pallet_xcm::Call::force_default_xcm_version { .. }
                | pallet_xcm::Call::force_subscribe_version_notify { .. }
                | pallet_xcm::Call::force_unsubscribe_version_notify { .. }
                | pallet_xcm::Call::send { .. } => true,
                _ => false,
            };
        }

        if let RuntimeCall::AssetsRegistry(ar_method) = call {
            return match ar_method {
                assets_registry::Call::force_mint { .. }
                | assets_registry::Call::force_burn { .. } => false,
                _ => true,
            };
        }

        if let RuntimeCall::Assets(assets_method) = call {
            return match assets_method {
                pallet_assets::Call::create { .. }
                | pallet_assets::Call::force_create { .. }
                | pallet_assets::Call::set_metadata { .. }
                | pallet_assets::Call::force_set_metadata { .. } => false,
                _ => true,
            };
        }

        if let RuntimeCall::Uniques(uniques_method) = call {
            return match uniques_method {
                pallet_uniques::Call::freeze { .. }
                | pallet_uniques::Call::thaw { .. }
                | pallet_uniques::Call::set_team { .. }
                | pallet_uniques::Call::set_accept_ownership { .. } => true,
                _ => false,
            };
        }

        if let RuntimeCall::RmrkCore(rmrk_core_method) = call {
            return match rmrk_core_method {
                pallet_rmrk_core::Call::change_collection_issuer { .. }
                | pallet_rmrk_core::Call::add_basic_resource { .. }
                | pallet_rmrk_core::Call::accept_resource { .. }
                | pallet_rmrk_core::Call::remove_resource { .. }
                | pallet_rmrk_core::Call::accept_resource_removal { .. }
                | pallet_rmrk_core::Call::send { .. } => true,
                _ => false,
            };
        }

        if let RuntimeCall::RmrkMarket(rmrk_market_method) = call {
            return match rmrk_market_method {
                pallet_rmrk_market::Call::buy { .. }
                | pallet_rmrk_market::Call::list { .. }
                | pallet_rmrk_market::Call::unlist { .. } => true,
                _ => false,
            };
        }

        matches!(
            call,
            // System
            RuntimeCall::System { .. } | RuntimeCall::Timestamp { .. } | RuntimeCall::Utility { .. } |
            RuntimeCall::Multisig { .. } | RuntimeCall::Proxy { .. } | RuntimeCall::Scheduler { .. } |
            RuntimeCall::Vesting { .. } | RuntimeCall::Preimage { .. } |
            // Parachain
            RuntimeCall::ParachainSystem { .. } |
            // Monetary
            RuntimeCall::Balances { .. }  |
            RuntimeCall::ChainBridge { .. } |
            RuntimeCall::XTransfer { .. } |
            // Collator
            RuntimeCall::CollatorSelection(_) | RuntimeCall::Session(_) |
            // XCM
            RuntimeCall::XcmpQueue { .. } |
            RuntimeCall::DmpQueue { .. } |
            // Governance
            RuntimeCall::Identity { .. } | RuntimeCall::Treasury { .. } |
            RuntimeCall::Democracy { .. } | //RuntimeCall::PhragmenElection { .. } |
            RuntimeCall::Council { .. } | RuntimeCall::TechnicalCommittee { .. } | RuntimeCall::TechnicalMembership { .. } |
            RuntimeCall::Bounties { .. } | RuntimeCall::ChildBounties { .. } |
            RuntimeCall::Lottery { .. } | RuntimeCall::Tips { .. } |
            // Phala
            RuntimeCall::PhalaMq { .. } | RuntimeCall::PhalaRegistry { .. } |
            RuntimeCall::PhalaComputation { .. } |
            RuntimeCall::PhalaStakePoolv2 { .. } | RuntimeCall::PhalaBasePool { .. } |
            RuntimeCall::PhalaWrappedBalances { .. } | RuntimeCall::PhalaVault { .. } |
            RuntimeCall::PhalaPhatContracts { .. } | RuntimeCall::PhalaPhatTokenomic { .. } |
            // inDEX
            RuntimeCall::PalletIndex { .. }
        )
    }
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 1200; // mortal tx can be valid up to 4 hour after signing
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 30;
}

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = Hasher;
    /// The header type.
    type Header = Header;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = RocksDbWeight;
    type BaseCallFilter = BaseCallFilter;
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 40);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const MaxProxies: u16 = 32;
    // One storage item; key size 32, value size 16
    pub const AnnouncementDepositBase: Balance = deposit(1, 48);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
    pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    RuntimeDebug,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    /// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
    Any,
    /// Can execute any call that does not transfer funds, including asset transfers.
    NonTransfer,
    /// Proxy with the ability to reject time-delay proxy announcements.
    CancelProxy,
    /// Governance
    Governance,
    /// Collator selection proxy. Can execute calls related to collator selection mechanism.
    Collator,
    /// Stake pool manager
    StakePoolManager,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}
impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => matches!(
                c,
                RuntimeCall::System { .. }
                    | RuntimeCall::Timestamp { .. }
                    | RuntimeCall::Session { .. }
                    | RuntimeCall::Democracy { .. }
                    | RuntimeCall::Council { .. }
                    | RuntimeCall::PhragmenElection { .. }
                    | RuntimeCall::TechnicalCommittee { .. }
                    | RuntimeCall::TechnicalMembership { .. }
                    | RuntimeCall::Treasury { .. }
                    | RuntimeCall::Bounties { .. }
                    | RuntimeCall::Tips { .. }
                    | RuntimeCall::Utility { .. }
                    | RuntimeCall::Identity { .. }
                    | RuntimeCall::Vesting(pallet_vesting::Call::vest { .. })
                    | RuntimeCall::Vesting(pallet_vesting::Call::vest_other { .. })
                    | RuntimeCall::Scheduler { .. }
                    | RuntimeCall::Proxy { .. }
                    | RuntimeCall::Multisig { .. }
            ),
            ProxyType::CancelProxy => matches!(
                c,
                RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                    | RuntimeCall::Utility { .. }
                    | RuntimeCall::Multisig { .. }
            ),
            ProxyType::Governance => matches!(
                c,
                RuntimeCall::Democracy { .. }
                    | RuntimeCall::PhragmenElection { .. }
                    | RuntimeCall::Council { .. }
                    | RuntimeCall::TechnicalCommittee { .. }
                    | RuntimeCall::Treasury { .. }
                    | RuntimeCall::Utility { .. }
                    | RuntimeCall::Bounties { .. }
                    | RuntimeCall::Tips { .. }
                    | RuntimeCall::Lottery { .. }
            ),
            ProxyType::Collator => matches!(
                c,
                RuntimeCall::CollatorSelection { .. }
                    | RuntimeCall::Utility { .. }
                    | RuntimeCall::Multisig { .. }
            ),
            ProxyType::StakePoolManager => matches!(
                c,
                RuntimeCall::Utility { .. }
                    | RuntimeCall::PhalaStakePoolv2(pallet_stake_pool_v2::Call::add_worker { .. })
                    | RuntimeCall::PhalaStakePoolv2(
                        pallet_stake_pool_v2::Call::remove_worker { .. }
                    )
                    | RuntimeCall::PhalaStakePoolv2(
                        pallet_stake_pool_v2::Call::start_computing { .. }
                    )
                    | RuntimeCall::PhalaStakePoolv2(
                        pallet_stake_pool_v2::Call::stop_computing { .. }
                    )
                    | RuntimeCall::PhalaStakePoolv2(
                        pallet_stake_pool_v2::Call::restart_computing { .. }
                    )
                    | RuntimeCall::PhalaStakePoolv2(
                        pallet_stake_pool_v2::Call::reclaim_pool_worker { .. }
                    )
                    | RuntimeCall::PhalaStakePoolv2(pallet_stake_pool_v2::Call::create { .. })
                    | RuntimeCall::PhalaRegistry(pallet_registry::Call::register_worker { .. })
                    | RuntimeCall::PhalaMq(pallet_mq::Call::sync_offchain_message { .. })
            ),
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (x, y) if x == y => true,
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::NonTransfer, _) => true,
            _ => false,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = MaxPending;
    type CallHasher = Hasher;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
    // Retry a scheduled item every 10 blocks (1 minute) until the preimage exists.
    pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRootOrHalfCouncil;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
}

parameter_types! {
    pub const PreimageMaxSize: u32 = 4096 * 1024;
    pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = 1 * CENTS;
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type BaseDeposit = PreimageBaseDeposit;
    type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * CENTS; // 0.01 PHA
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const AssetDeposit: Balance = 1 * CENTS; // 1 CENTS deposit to create asset
    pub const ApprovalDeposit: Balance = 1 * CENTS;
    pub const AssetsStringLimit: u32 = 50;
    pub const AssetAccountDeposit: u128 = 1 * DOLLARS;
    /// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
    // https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = AssetsStringLimit;
    type RemoveItemsLimit = ConstU32<1000>;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1 * MILLICENTS;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub const OperationalFeeMultiplier: u8 = 5;
    pub AdjustmentVariable: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub MaximumMultiplier: pallet_transaction_payment::Multiplier = Bounded::max_value();
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            Treasury::on_unbalanced(fees);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = pallet_transaction_payment::TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl pallet_tips::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type Tippers = PhragmenElection;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type RegistrarOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 1 * CENTS; // 0.01 PHA
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const LotteryPalletId: PalletId = PalletId(*b"py/lotto");
    pub const MaxCalls: u32 = 10;
    pub const MaxGenerateRandom: u32 = 10;
}

impl pallet_lottery::Config for Runtime {
    type PalletId = LotteryPalletId;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Randomness = RandomnessCollectiveFlip;
    type ManagerOrigin = EnsureRootOrHalfCouncil;
    type MaxCalls = MaxCalls;
    type ValidateCall = Lottery;
    type MaxGenerateRandom = MaxGenerateRandom;
    type WeightInfo = pallet_lottery::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = pallet_parachain_info::Pallet<Runtime>;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

parameter_types! {
    pub const CollectionDeposit: Balance = 0 * DOLLARS;
    pub const ItemDeposit: Balance = 0 * DOLLARS;
    pub const ZeroDeposit: Balance = 0 * DOLLARS;
}

impl pallet_uniques::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type Locker = RmrkCore;
    type CollectionDeposit = CollectionDeposit;
    type ItemDeposit = ItemDeposit;
    type MetadataDepositBase = ZeroDeposit;
    type AttributeDepositBase = ZeroDeposit;
    type DepositPerByte = ZeroDeposit;
    type StringLimit = uniques::StringLimit;
    type KeyLimit = uniques::KeyLimit;
    type ValueLimit = uniques::ValueLimit;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    type WeightInfo = pallet_uniques::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ResourceSymbolLimit: u32 = 10;
    pub const MaxPriorities: u32 = 25;
    pub const PropertiesLimit: u32 = 15;
    pub const MaxResourcesOnMint: u32 = 100;
    pub const NestingBudget: u32 = 200;
}

impl pallet_rmrk_core::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ProtocolOrigin = EnsureRoot<AccountId>;
    type ResourceSymbolLimit = ResourceSymbolLimit;
    type PartsLimit = rmrk_core::PartsLimit;
    type MaxPriorities = MaxPriorities;
    type PropertiesLimit = PropertiesLimit;
    type NestingBudget = NestingBudget;
    type CollectionSymbolLimit = rmrk_core::CollectionSymbolLimit;
    type MaxResourcesOnMint = MaxResourcesOnMint;
    type WeightInfo = pallet_rmrk_core::weights::SubstrateWeight<Runtime>;
    type TransferHooks = PhalaWrappedBalances;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = pallet_rmrk_core::RmrkBenchmark;
}

impl pallet_rmrk_equip::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxPropertiesPerTheme = rmrk_equip::MaxPropertiesPerTheme;
    type MaxCollectionsEquippablePerPart = rmrk_equip::MaxCollectionsEquippablePerPart;
    type WeightInfo = pallet_rmrk_equip::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MinimumOfferAmount: Balance = DOLLARS / 10_000;
    pub const MarketFee: Permill = Permill::from_parts(5_000);
}

impl pallet_rmrk_market::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ProtocolOrigin = EnsureRoot<AccountId>;
    type Currency = Balances;
    type MinimumOfferAmount = MinimumOfferAmount;
    type WeightInfo = pallet_rmrk_market::weights::SubstrateWeight<Runtime>;
    type MarketplaceHooks = ();
    type MarketFee = MarketFee;
}

impl pallet_parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CandidacyBond: Balance = 10 * DOLLARS;
    // 1 storage item created, key size is 32 bytes, value size is 16+16.
    pub const VotingBondBase: Balance = deposit(1, 64);
    // additional data per vote is 32 bytes (account id).
    pub const VotingBondFactor: Balance = deposit(0, 32);
    /// Daily council elections
    pub const TermDuration: BlockNumber = 24 * HOURS;
    pub const DesiredMembers: u32 = 8;
    pub const DesiredRunnersUp: u32 = 8;
    pub const MaxVotesPerVoter: u32 = 16;
    pub const MaxVoters: u32 = 500;
    pub const MaxCandidates: u32 = 20;
    pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than MaxMembers members elected via phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ChangeMembers = Council;
    type InitializeMembers = Council;
    type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = Treasury;
    type KickedMember = Treasury;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type MaxVoters = MaxVoters;
    type MaxCandidates = MaxCandidates;
    type MaxVotesPerVoter = MaxVotesPerVoter;
    type PalletId = PhragmenElectionPalletId;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type SetMembersOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ChildBountyValueMinimum: Balance = 1 * DOLLARS;
    pub const MaxActiveChildBountyCount: u32 = 5;
}

impl pallet_child_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
    type ChildBountyValueMinimum = ChildBountyValueMinimum;
    type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyValueMinimum: Balance = 5 * DOLLARS;
    pub const BountyDepositBase: Balance = 1 * DOLLARS;
    pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
    pub const CuratorDepositMin: Balance = 1 * DOLLARS;
    pub const CuratorDepositMax: Balance = 100 * DOLLARS;
    pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
}

impl pallet_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type CuratorDepositMultiplier = CuratorDepositMultiplier;
    type CuratorDepositMin = CuratorDepositMin;
    type CuratorDepositMax = CuratorDepositMax;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
    type ChildBountyManager = ChildBounties;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const Burn: Permill = Permill::zero();
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * DOLLARS;
    pub const DataDepositPerByte: Balance = 1 * CENTS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const MaximumReasonLength: u32 = 16384;
    pub const MaxApprovals: u32 = 100;
    pub const MaxBalance: Balance = Balance::max_value();
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
    >;
    type RejectOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
    >;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ();
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
    type SpendOrigin = frame_system::EnsureWithSuccess<
        frame_support::traits::EitherOf<
            EnsureRoot<AccountId>,
            pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
        >,
        AccountId, MaxBalance,
    >;
}

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 7 * DAYS;
    pub const VotingPeriod: BlockNumber = 7 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    pub const InstantAllowed: bool = true;
    pub const MinimumDeposit: Balance = 10 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 8 * DAYS;
    pub const CooloffPeriod: BlockNumber = 7 * DAYS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
    pub const MaxDeposits: u32 = 100;
    pub const MaxBlacklisted: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
        EnsureRoot<AccountId>,
    >;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
        EnsureRoot<AccountId>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
    type Preimages = Preimage;
    type MaxDeposits = MaxDeposits;
    type MaxBlacklisted = MaxBlacklisted;
}

parameter_types! {
    pub const MaxAuthorities: u32 = 100;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = CollatorSelection;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but lets be pedantic.
    type SessionHandler =
        <opaque::SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = opaque::SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const MaxCollatorCandidates: u32 = 1000;
    pub const MinCollatorCandidates: u32 = 5;
    pub const SessionLength: BlockNumber = 6 * HOURS;
    pub const MaxInvulnerables: u32 = 100;
}

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = EnsureRootOrHalfCouncil;
    type PotId = PotId;
    type MaxCandidates = MaxCollatorCandidates;
    type MinCandidates = MinCollatorCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = pallet_collator_selection::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorMultiLocation =
        X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);
parameter_types! {
    pub UnitWeightCost: XCMWeight = XCMWeight::from_parts(200_000_000u64, 0);
    pub const MaxInstructions: u32 = 100;
    pub PhalaTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
    pub CheckingAccountForCurrencyAdapter: Option<(AccountId, MintLocation)> = None;
    pub CheckingAccountForFungibleAdapter: AccountId = PalletId(*b"checking").into_account_truncating();
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Subscriptions for version tracking are OK.
    AllowSubscriptionsFrom<Everything>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    helper::NativeAssetMatcher<assets_registry::NativeAssetFilter<ParachainInfo>>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    CheckingAccountForCurrencyAdapter,
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    helper::ConcreteAssetsMatcher<
        <Runtime as pallet_assets::Config>::AssetId,
        Balance,
        AssetsRegistry,
    >,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We do not support teleport assets
    NoChecking,
    // We do not support teleport assets
    CheckingAccountForFungibleAdapter,
>;

parameter_types! {
    pub NativeExecutionPrice: u128 = pha_per_second();
    pub WeightPerSecond: u64 = WEIGHT_REF_TIME_PER_SECOND;
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = XTransferAdapter<
        CurrencyTransactor,
        FungiblesTransactor,
        XTransfer,
        assets_registry::NativeAssetFilter<ParachainInfo>,
        assets_registry::ReserveAssetFilter<
            ParachainInfo,
            assets_registry::NativeAssetFilter<ParachainInfo>,
        >,
    >;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = helper::AssetOriginFilter;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = DynamicWeightTrader<
        WeightPerSecond,
        <Runtime as pallet_assets::Config>::AssetId,
        AssetsRegistry,
        helper::XTransferTakeRevenue<Self::AssetTransactor, AccountId, PhalaTreasuryAccount>,
    >;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<BaseCallFilter>;
    type SafeCallFilter = BaseCallFilter;
}
parameter_types! {
    pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(10);
}
/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}
impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRootOrHalfCouncil;
    type ControllerOrigin = EnsureRootOrHalfCouncil;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
    type PriceForSiblingDelivery = ();
}
impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = EnsureRootOrHalfCouncil;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}
impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// No local origins on this chain are allowed to dispatch XCM sends.
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = ();
    type MaxLockers = ConstU32<8>;
    type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
}

impl xcmbridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
    type AssetsRegistry = AssetsRegistry;
}

parameter_types! {
    pub PHALocation: MultiLocation = MultiLocation::here();
    pub PHASygmaResourceId: [u8; 32] = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000001");
}
impl assets_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RegistryCommitteeOrigin = EnsureRootOrHalfCouncil;
    type Currency = Balances;
    type MinBalance = ExistentialDeposit;
    type NativeExecutionPrice = NativeExecutionPrice;
    type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
    type ReserveAssetChecker = assets_registry::ReserveAssetFilter<
        ParachainInfo,
        assets_registry::NativeAssetFilter<ParachainInfo>,
    >;
    type ResourceIdGenerationSalt = ResourceIdGenerationSalt;
    type NativeAssetLocation = PHALocation;
    type NativeAssetSygmaResourceId = PHASygmaResourceId;
}

parameter_types! {
    pub const BridgeChainId: u8 = 3;
    pub const ResourceIdGenerationSalt: Option<u128> = Some(3);
    pub const ProposalLifetime: BlockNumber = 50400; // ~7 days
    pub const BridgeEventLimit: u32 = 1024;
}

impl chainbridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BridgeCommitteeOrigin = EnsureRootOrHalfCouncil;
    type Proposal = RuntimeCall;
    type BridgeChainId = BridgeChainId;
    type Currency = Balances;
    type ProposalLifetime = ProposalLifetime;
    type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
    type NativeExecutionPrice = NativeExecutionPrice;
    type TreasuryAccount = PhalaTreasuryAccount;
    type FungibleAdapter = XTransferAdapter<
        CurrencyTransactor,
        FungiblesTransactor,
        XTransfer,
        assets_registry::NativeAssetFilter<ParachainInfo>,
        assets_registry::ReserveAssetFilter<
            ParachainInfo,
            assets_registry::NativeAssetFilter<ParachainInfo>,
        >,
    >;
    type AssetsRegistry = AssetsRegistry;
    type BridgeEventLimit = BridgeEventLimit;
    type ResourceIdGenerationSalt = ResourceIdGenerationSalt;
}

impl xtransfer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Bridge = (
        xcmbridge::BridgeTransactImpl<Runtime>,
        chainbridge::BridgeTransactImpl<Runtime>,
    );
}

pub struct MqCallMatcher;
impl pallet_mq::CallMatcher<Runtime> for MqCallMatcher {
    fn match_call(call: &RuntimeCall) -> Option<&pallet_mq::Call<Runtime>> {
        match call {
            RuntimeCall::PhalaMq(mq_call) => Some(mq_call),
            _ => None,
        }
    }
}

parameter_types! {
    pub const ExpectedBlockTimeSec: u32 = SECS_PER_BLOCK as u32;
    pub const MinWorkingStaking: Balance = 1 * DOLLARS;
    pub const MinContribution: Balance = 1 * CENTS;
    pub const WorkingGracePeriod: u64 = 7 * 24 * 3600;
    pub const MinInitP: u32 = 50;
    pub const ComputingEnabledByDefault: bool = false;
    pub const MaxPoolWorkers: u32 = 200;
    pub const NoneAttestationEnabled: bool = false;
    pub const VerifyPRuntime: bool = true;
    pub const VerifyRelaychainGenesisBlockHash: bool = true;
    pub ParachainId: u32 = ParachainInfo::parachain_id().into();
}

impl pallet_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UnixTime = Timestamp;
    type LegacyAttestationValidator = pallet_registry::IasValidator;
    type NoneAttestationEnabled = NoneAttestationEnabled;
    type VerifyPRuntime = VerifyPRuntime;
    type VerifyRelaychainGenesisBlockHash = VerifyRelaychainGenesisBlockHash;
    type GovernanceOrigin = EnsureRootOrHalfCouncil;
    type ParachainId = ParachainId;
}
impl pallet_mq::Config for Runtime {
    type QueueNotifyConfig = msg_routing::MessageRouteConfig;
    type CallMatcher = MqCallMatcher;
}

pub struct SetBudgetMembers;

impl SortedMembers<AccountId> for SetBudgetMembers {
    fn sorted_members() -> Vec<AccountId> {
        [pallet_computation::pallet::ContractAccount::<Runtime>::get()].to_vec()
    }
}

impl pallet_computation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ExpectedBlockTimeSec = ExpectedBlockTimeSec;
    type MinInitP = MinInitP;
    type Randomness = RandomnessCollectiveFlip;
    type OnReward = PhalaStakePoolv2;
    type OnUnbound = PhalaStakePoolv2;
    type OnStopped = PhalaStakePoolv2;
    type OnTreasurySettled = Treasury;
    type UpdateTokenomicOrigin = EnsureRootOrHalfCouncil;
    type SetBudgetOrigins = EnsureSignedBy<SetBudgetMembers, AccountId>;
    type SetContractRootOrigins = EnsureRootOrHalfCouncil;
}
impl pallet_stake_pool_v2::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MinContribution = MinContribution;
    type GracePeriod = WorkingGracePeriod;
    type ComputingEnabledByDefault = ComputingEnabledByDefault;
    type MaxPoolWorkers = MaxPoolWorkers;
    type ComputingSwitchOrigin = EnsureRootOrHalfCouncil;
}
impl pallet_stake_pool::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
}

parameter_types! {
    pub const InitialPriceCheckPoint: Balance = 1 * DOLLARS;
    pub const WPhaMinBalance: Balance = CENTS;
}

impl pallet_vault::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type InitialPriceCheckPoint = InitialPriceCheckPoint;
}

pub struct WrappedBalancesPalletAccount;

impl Get<AccountId32> for WrappedBalancesPalletAccount {
    fn get() -> AccountId32 {
        (b"wpha/")
            .using_encoded(|b| AccountId32::decode(&mut TrailingZeroInput::new(b)))
            .expect("Decoding zero-padded account id should always succeed; qed")
    }
}

impl pallet_wrapped_balances::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WPhaAssetId = ConstU32<10000>;
    type WrappedBalancesAccountId = WrappedBalancesPalletAccount;
    type OnSlashed = Treasury;
}

pub struct MigrationAccount;

impl Get<AccountId32> for MigrationAccount {
    fn get() -> AccountId32 {
        let account: [u8; 32] =
            hex_literal::hex!("9e6399cd577e8ac536bdc017675f747b2d1893ad9cc8c69fd17eef73d4e6e51e");
        account.into()
    }
}

impl pallet_base_pool::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MigrationAccountId = MigrationAccount;
    type WPhaMinBalance = WPhaMinBalance;
}

impl phala_pallets::PhalaConfig for Runtime {
    type Currency = Balances;
}

impl pallet_phat::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type InkCodeSizeLimit = ConstU32<{1024*1024*2}>;
    type SidevmCodeSizeLimit = ConstU32<{1024*1024*8}>;
    type Currency = Balances;
}
impl pallet_phat_tokenomic::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
}

impl pallet_index::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CommitteeOrigin = EnsureRootOrHalfCouncil;
    type AssetTransactor = (CurrencyTransactor, FungiblesTransactor);
    type AssetsRegistry = AssetsRegistry;
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_preimage, Preimage]
        [pallet_bounties, Bounties]
        [pallet_child_bounties, ChildBounties]
        [pallet_collective, Council]
        [pallet_collective, TechnicalCommittee]
        [pallet_democracy, Democracy]
        // TODO: assertion failed `failed to submit candidacy`
        [pallet_elections_phragmen, PhragmenElection]
        [pallet_identity, Identity]
        [pallet_membership, TechnicalMembership]
        [pallet_multisig, Multisig]
        [pallet_proxy, Proxy]
        [pallet_scheduler, Scheduler]
        [pallet_session, SessionBench::<Runtime>]
        [pallet_timestamp, Timestamp]
        [pallet_tips, Tips]
        [pallet_treasury, Treasury]
        [pallet_utility, Utility]
        [pallet_vesting, Vesting]
        [pallet_lottery, Lottery]
        [pallet_assets, Assets]
        // TODO: panic
        [pallet_collator_selection, CollatorSelection]
        [pallet_uniques, Uniques]
    );
}

// https://github.com/rmrk-team/rmrk-substrate/blob/main/runtime/src/lib.rs#L472
fn option_filter_keys_to_set<StringLimit: frame_support::traits::Get<u32>>(
    filter_keys: Option<Vec<pallet_rmrk_rpc_runtime_api::PropertyKey>>,
) -> pallet_rmrk_rpc_runtime_api::Result<Option<BTreeSet<BoundedVec<u8, StringLimit>>>> {
    match filter_keys {
        Some(filter_keys) => {
            let tree = filter_keys.into_iter()
                .map(|filter_keys| -> pallet_rmrk_rpc_runtime_api::Result<BoundedVec<u8, StringLimit>> {
                    filter_keys.try_into().map_err(|_| DispatchError::Other("Can't read filter key"))
                }).collect::<pallet_rmrk_rpc_runtime_api::Result<BTreeSet<_>>>()?;
            Ok(Some(tree))
        }
        None => Ok(None),
    }
}

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(
            extrinsic: <Block as BlockT>::Extrinsic,
        ) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_mq_runtime_api::MqApi<Block> for Runtime {
        fn sender_sequence(sender: &phala_types::messaging::MessageOrigin) -> Option<u64> {
            PhalaMq::offchain_ingress(sender)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    impl pallet_rmrk_rpc_runtime_api::RmrkApi<
        Block,
        AccountId,
        CollectionInfoOf<Runtime>,
        InstanceInfoOf<Runtime>,
        ResourceInfoOf<Runtime>,
        PropertyInfoOf<Runtime>,
        BaseInfoOf<Runtime>,
        PartTypeOf<Runtime>,
        BoundedThemeOf<Runtime>
    > for Runtime
    {
        fn collection_by_id(id: CollectionId) -> pallet_rmrk_rpc_runtime_api::Result<Option<CollectionInfoOf<Runtime>>> {
            Ok(RmrkCore::collections(id))
        }

        fn nft_by_id(collection_id: CollectionId, nft_id: NftId) -> pallet_rmrk_rpc_runtime_api::Result<Option<InstanceInfoOf<Runtime>>> {
            Ok(RmrkCore::nfts(collection_id, nft_id))
        }

        fn account_tokens(account_id: AccountId, collection_id: CollectionId) -> pallet_rmrk_rpc_runtime_api::Result<Vec<NftId>> {
            Ok(Uniques::owned_in_collection(&collection_id, &account_id).collect())
        }

        fn nft_children(collection_id: CollectionId, nft_id: NftId) -> pallet_rmrk_rpc_runtime_api::Result<Vec<NftChild<CollectionId, NftId>>> {
            let children = RmrkCore::iterate_nft_children(collection_id, nft_id).collect();

            Ok(children)
        }

        fn collection_properties(
            collection_id: CollectionId,
            filter_keys: Option<Vec<pallet_rmrk_rpc_runtime_api::PropertyKey>>
        ) -> pallet_rmrk_rpc_runtime_api::Result<Vec<PropertyInfoOf<Runtime>>> {
            let nft_id = None;

            let filter_keys = option_filter_keys_to_set::<<Self as pallet_uniques::Config>::KeyLimit>(
                filter_keys
            )?;

            Ok(RmrkCore::query_properties(collection_id, nft_id, filter_keys).collect())
        }

        fn nft_properties(
            collection_id: CollectionId,
            nft_id: NftId,
            filter_keys: Option<Vec<pallet_rmrk_rpc_runtime_api::PropertyKey>>
        ) -> pallet_rmrk_rpc_runtime_api::Result<Vec<PropertyInfoOf<Runtime>>> {
            let filter_keys = option_filter_keys_to_set::<<Self as pallet_uniques::Config>::KeyLimit>(
                filter_keys
            )?;

            Ok(RmrkCore::query_properties(collection_id, Some(nft_id), filter_keys).collect())
        }

        fn nft_resources(collection_id: CollectionId, nft_id: NftId) -> pallet_rmrk_rpc_runtime_api::Result<Vec<ResourceInfoOf<Runtime>>> {
            Ok(RmrkCore::iterate_resources(collection_id, nft_id).collect())
        }

        fn nft_resource_priority(collection_id: CollectionId, nft_id: NftId, resource_id: ResourceId) -> pallet_rmrk_rpc_runtime_api::Result<Option<u32>> {
            let priority = RmrkCore::priorities((collection_id, nft_id, resource_id));

            Ok(priority)
        }

        fn base(base_id: BaseId) -> pallet_rmrk_rpc_runtime_api::Result<Option<BaseInfoOf<Runtime>>> {
            Ok(RmrkEquip::bases(base_id))
        }

        fn base_parts(base_id: BaseId) -> pallet_rmrk_rpc_runtime_api::Result<Vec<PartTypeOf<Runtime>>> {
            Ok(RmrkEquip::iterate_part_types(base_id).collect())
        }

        fn theme_names(base_id: BaseId) -> pallet_rmrk_rpc_runtime_api::Result<Vec<pallet_rmrk_rpc_runtime_api::ThemeName>> {
            let names = RmrkEquip::iterate_theme_names(base_id)
                .map(|name| name.into())
                .collect();

            Ok(names)
        }

        fn theme(
            base_id: BaseId,
            theme_name: pallet_rmrk_rpc_runtime_api::ThemeName,
            filter_keys: Option<Vec<pallet_rmrk_rpc_runtime_api::PropertyKey>>
        ) -> pallet_rmrk_rpc_runtime_api::Result<Option<BoundedThemeOf<Runtime>>> {
            use pallet_rmrk_equip::StringLimitOf;

            let theme_name: StringLimitOf<Self> = theme_name.try_into()
                .map_err(|_| DispatchError::Other("Can't read theme_name"))?;

            let filter_keys = option_filter_keys_to_set::<<Self as pallet_uniques::Config>::StringLimit>(
                filter_keys
            )?;

            let theme = RmrkEquip::get_theme(base_id, theme_name, filter_keys)?;
            Ok(theme)
        }
    }

    impl sygma_runtime_api::SygmaBridgeApi<Block> for Runtime {
        fn is_proposal_executed(nonce: DepositNonce, domain_id: DomainID) -> bool {
            // TODO: enable Sygma
            false
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            use frame_system_benchmarking::Pallet as SystemBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            impl cumulus_pallet_session_benchmarking::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
                // Treasury Account
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
    fn check_inherents(
        block: &Block,
        relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
    ) -> sp_inherents::CheckInherentsResult {
        let relay_chain_slot = relay_state_proof
            .read_slot()
            .expect("Could not read the relay chain slot from the proof");

        let inherent_data =
            cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
                relay_chain_slot,
                sp_std::time::Duration::from_secs(6),
            )
            .create_inherent_data()
            .expect("Could not create the timestamp inherent data");

        inherent_data.check_extrinsics(block)
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
    CheckInherents = CheckInherents,
}
