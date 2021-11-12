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

//! Khala runtime.

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

// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, fee::WeightToFee, parachains};

mod msg_routing;

use codec::{Decode, Encode, MaxEncodedLen};
use sp_api::impl_runtime_apis;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4, _5},
    OpaqueMetadata,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdLookup, Block as BlockT, ConvertInto, Zero},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, match_type, parameter_types,
    traits::{
        Contains, Currency, Everything, Imbalance, InstanceFilter, IsInVec, KeyOwnerProofSystem,
        LockIdentifier, OnUnbalanced, Randomness, U128CurrencyToVote,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    PalletId, RuntimeDebug, StorageValue,
};

use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureOneOf, EnsureRoot,
};

use xcm::{v1::prelude::*, Version as XcmVersion};
use polkadot_parachain::primitives::Sibling;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
    AllowUnpaidExecutionFrom, AsPrefixedGeneralIndex, ConvertedConcreteAssetId, CurrencyAdapter, EnsureXcmOrigin,
    FixedWeightBounds, FungiblesAdapter, IsConcrete, LocationInverter, ParentAsSuperuser,
    ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
    SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
    UsingComponents,
};
use xcm_executor::{traits::{JustTry, FilterAssetLocation}, Config, XcmExecutor};

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;

pub use parachains_common::*;
pub use parachains_common::Index;

pub use phala_pallets::{pallet_mining, pallet_mq, pallet_registry, pallet_stakepool};

pub use xtransfer_pallets::{
    pallet_xtransfer_assets,
    pallet_xcm_transfer,
    xcm_helper,
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
    spec_name: create_runtime_str!("khala"),
    impl_name: create_runtime_str!("khala"),
    authoring_version: 1,
    spec_version: 1070,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 2,
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
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_mq::CheckMqSequence<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
>;

construct_runtime! {
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        // System support stuff
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 2,
        Utility: pallet_utility::{Pallet, Call, Event} = 3,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 4,
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 5,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 6,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 7,

        // Parachain staff
        ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config} = 20,
        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned} = 21,

        // XCM helpers
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 31,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 32,

        // Monetary stuff
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 40,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 41,

        // Collator support. the order of these 5 are important and shall not change.
        Authorship: pallet_authorship::{Pallet, Call, Storage} = 50,
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

        // Main, starts from 80

        // ChainBridge
        ChainBridge: pallet_bridge::{Pallet, Call, Storage, Event<T>} = 80,
        BridgeTransfer: pallet_bridge_transfer::{Pallet, Call, Event<T>, Storage} = 81,
        XcmTransfer: pallet_xcm_transfer::{Pallet, Call, Event<T>, Storage} = 82,
        XTransferAssets: pallet_xtransfer_assets::{Pallet, Call, Event<T>, Storage} = 83,

        // Phala
        PhalaMq: pallet_mq::{Pallet, Call, Storage} = 85,
        PhalaRegistry: pallet_registry::{Pallet, Call, Event, Storage, Config<T>} = 86,
        PhalaMining: pallet_mining::{Pallet, Call, Event<T>, Storage, Config} = 87,
        PhalaStakePool: pallet_stakepool::{Pallet, Call, Event<T>, Storage} = 88,

        // `sudo` has been removed on production
        // Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 99,
        // `OTT` has been removed, the index should be kept
        // PhalaOneshotTransfer: pallet_ott::{Pallet, Call, Event<T>, Storage} = 100,
    }
}

pub struct BaseCallFilter;
impl Contains<Call> for BaseCallFilter {
    fn contains(call: &Call) -> bool {
        matches!(
            call,
            // `sudo` has been removed on production
            // Call::Sudo { .. } |
            // System
            Call::System { .. } | Call::Timestamp { .. } | Call::Utility { .. } |
            Call::Multisig { .. } | Call::Proxy { .. } | Call::Scheduler { .. } |
            Call::Vesting { .. } |
            // Parachain
            Call::ParachainSystem { .. } |
            // Monetary
            Call::Balances { .. }  |
            Call::ChainBridge { .. } |
            Call::BridgeTransfer { .. } |
            // Collator
            Call::Authorship { .. } | Call::CollatorSelection { .. } | Call::Session { .. } |
            // XCM
            Call::XcmpQueue { .. } |
            Call::DmpQueue { .. } |
            Call::XcmTransfer { .. } |
            Call::XTransferAssets { .. } |
            // Governance
            Call::Identity { .. } | Call::Treasury { .. } |
            Call::Democracy { .. } | Call::PhragmenElection { .. } |
            Call::Council { .. } | Call::TechnicalCommittee { .. } | Call::TechnicalMembership { .. } |
            Call::Bounties { .. } | Call::Lottery { .. } | Call::Tips { .. } |
            // Phala
            Call::PhalaMq { .. } | Call::PhalaRegistry { .. } |
            Call::PhalaMining { .. } | Call::PhalaStakePool { .. }
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
    type Call = Call;
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
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
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
}

impl pallet_randomness_collective_flip::Config for Runtime {}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
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
    type Event = Event;
    type Call = Call;
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
impl InstanceFilter<Call> for ProxyType {
    fn filter(&self, c: &Call) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => matches!(
                c,
                Call::System { .. }
                    | Call::Timestamp { .. }
                    | Call::Session { .. }
                    | Call::Democracy { .. }
                    | Call::Council { .. }
                    | Call::PhragmenElection { .. }
                    | Call::TechnicalCommittee { .. }
                    | Call::TechnicalMembership { .. }
                    | Call::Treasury { .. }
                    | Call::Bounties { .. }
                    | Call::Tips { .. }
                    | Call::Utility { .. }
                    | Call::Identity { .. }
                    | Call::Vesting(pallet_vesting::Call::vest { .. })
                    | Call::Vesting(pallet_vesting::Call::vest_other { .. })
                    | Call::Scheduler { .. }
                    | Call::Proxy { .. }
                    | Call::Multisig { .. }
            ),
            ProxyType::CancelProxy => matches!(
                c,
                Call::Proxy(pallet_proxy::Call::reject_announcement { .. })
                    | Call::Utility { .. }
                    | Call::Multisig { .. }
            ),
            ProxyType::Governance => matches!(
                c,
                Call::Democracy { .. }
                    | Call::PhragmenElection { .. }
                    | Call::Council { .. }
                    | Call::TechnicalCommittee { .. }
                    | Call::Treasury { .. }
                    | Call::Utility { .. }
                    | Call::Bounties { .. }
                    | Call::Tips { .. }
                    | Call::Lottery { .. }
            ),
            ProxyType::Collator => matches!(
                c,
                Call::CollatorSelection { .. } | Call::Utility { .. } | Call::Multisig { .. }
            ),
            ProxyType::StakePoolManager => matches!(
                c,
                Call::Utility { .. }
                    | Call::PhalaStakePool(pallet_stakepool::Call::add_worker { .. })
                    | Call::PhalaStakePool(pallet_stakepool::Call::remove_worker { .. })
                    | Call::PhalaStakePool(pallet_stakepool::Call::start_mining { .. })
                    | Call::PhalaStakePool(pallet_stakepool::Call::stop_mining { .. })
                    | Call::PhalaStakePool(pallet_stakepool::Call::reclaim_pool_worker { .. })
                    | Call::PhalaStakePool(pallet_stakepool::Call::create { .. })
                    | Call::PhalaRegistry(pallet_registry::Call::register_worker { .. })
                    | Call::PhalaMq(pallet_mq::Call::sync_offchain_message { .. })
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
    type Event = Event;
    type Call = Call;
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
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRootOrHalfCouncil;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
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
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1 * MILLICENTS;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub const OperationalFeeMultiplier: u8 = 5;
    pub AdjustmentVariable: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 1_000_000_000u128);
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
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate = pallet_transaction_payment::TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
    >;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

impl pallet_bounties::Config for Runtime {
    type Event = Event;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyCuratorDeposit = BountyCuratorDeposit;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_tips::Config for Runtime {
    type Event = Event;
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
    type Event = Event;
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
}

impl pallet_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const LotteryPalletId: PalletId = PalletId(*b"py/lotto");
    pub const MaxCalls: u32 = 10;
    pub const MaxGenerateRandom: u32 = 10;
}

impl pallet_lottery::Config for Runtime {
    type PalletId = LotteryPalletId;
    type Call = Call;
    type Event = Event;
    type Currency = Balances;
    type Randomness = RandomnessCollectiveFlip;
    type ManagerOrigin = EnsureRootOrHalfCouncil;
    type MaxCalls = MaxCalls;
    type ValidateCall = Lottery;
    type MaxGenerateRandom = MaxGenerateRandom;
    type WeightInfo = pallet_lottery::weights::SubstrateWeight<Runtime>;
}

// `sudo` has been removed on production
// impl pallet_sudo::Config for Runtime {
//     type Call = Call;
//     type Event = Event;
// }

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = pallet_parachain_info::Pallet<Runtime>;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl pallet_parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const KsmLocation: MultiLocation = MultiLocation::parent();
    pub const RelayNetwork: NetworkId = NetworkId::Kusama;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
    pub const DOTMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: Here };
    pub KARMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::KAR_KEY.to_vec())) };
    // TODO: would be removed when ready to release
    pub const PHA2004MultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(2004)) };
    pub const PHA2005MultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(2005)) };
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
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
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
);
parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
}
match_type! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}
pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
    // TODO: would be removed when we ready to release, it's unreasonable to let Everything execute without pay.
	AllowUnpaidExecutionFrom<Everything>,
	// Expected responses are OK.
	// AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = XTransferAssets;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = xcm_helper::AssetOriginFilter;
	type IsTeleporter = (); // <- should be enough to allow teleportation of KSM
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = (
        UsingComponents<IdentityFee<Balance>, DOTMultiAssetId, AccountId, Balances, ()>,
        UsingComponents<IdentityFee<Balance>, KARMultiAssetId, AccountId, Balances, ()>,
        UsingComponents<IdentityFee<Balance>, PHA2004MultiAssetId, AccountId, Balances, ()>,
        UsingComponents<IdentityFee<Balance>, PHA2005MultiAssetId, AccountId, Balances, ()>,
    );
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
}
parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}
/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
}
impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

impl pallet_xcm_transfer::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
}

impl pallet_xtransfer_assets::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
    type XTransferCommitteeOrigin = EnsureRootOrHalfCouncil;
    type FungibleMatcher = xcm_helper::IsSiblingParachainsConcrete<XTransferAssets>;
    type AccountIdConverter = LocationToAccountId;
    type ParachainInfo = ParachainInfo;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
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
    pub const DesiredMembers: u32 = 5;
    pub const DesiredRunnersUp: u32 = 5;
    pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than MaxMembers members elected via phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type Event = Event;
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
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

type EnsureRootOrHalfCouncil = EnsureOneOf<
    AccountId,
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;
impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type Event = Event;
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
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const Burn: Permill = Permill::zero();
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * DOLLARS;
    pub const DataDepositPerByte: Balance = 1 * CENTS;
    pub const BountyDepositBase: Balance = 1 * DOLLARS;
    pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
    pub const MaximumReasonLength: u32 = 16384;
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyValueMinimum: Balance = 5 * DOLLARS;
    pub const MaxApprovals: u32 = 100;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
    >;
    type RejectOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
    >;
    type Event = Event;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
}

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 7 * DAYS;
    pub const VotingPeriod: BlockNumber = 7 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    pub const InstantAllowed: bool = true;
    pub const MinimumDeposit: Balance = 10 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 8 * DAYS;
    pub const CooloffPeriod: BlockNumber = 7 * DAYS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = 1 * CENTS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin = frame_system::EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = frame_system::EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = frame_system::EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = frame_system::EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantOrigin = frame_system::EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin = EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>,
        EnsureRoot<AccountId>,
    >;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureOneOf<
        AccountId,
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
        EnsureRoot<AccountId>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
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
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = CollatorSelection;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type Event = Event;
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
    pub const MaxCandidates: u32 = 1000;
    pub const MinCandidates: u32 = 5;
    pub const SessionLength: BlockNumber = 6 * HOURS;
    pub const MaxInvulnerables: u32 = 100;
}

impl pallet_collator_selection::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type UpdateOrigin = EnsureRootOrHalfCouncil;
    type PotId = PotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = pallet_collator_selection::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BridgeChainId: u8 = 1;
    pub const ProposalLifetime: BlockNumber = 50400; // ~7 days
}

impl pallet_bridge::Config for Runtime {
    type Event = Event;
    type BridgeCommitteeOrigin = EnsureRootOrHalfCouncil;
    type Proposal = Call;
    type BridgeChainId = BridgeChainId;
    type ProposalLifetime = ProposalLifetime;
}

parameter_types! {
    // bridge::derive_resource_id(1, &bridge::hashing::blake2_128(b"PHA"));
    pub const NativeTokenResourceId: [u8; 32] = hex_literal::hex!("00000000000000000000000000000063a7e2be78898ba83824b0c0cc8dfb6001");
}

impl pallet_bridge_transfer::Config for Runtime {
    type Event = Event;
    type BridgeOrigin = pallet_bridge::EnsureBridge<Runtime>;
    type Currency = Balances;
    type NativeTokenResourceId = NativeTokenResourceId;
    type OnFeePay = Treasury;
}

pub struct MqCallMatcher;
impl pallet_mq::CallMatcher<Runtime> for MqCallMatcher {
    fn match_call(call: &Call) -> Option<&pallet_mq::Call<Runtime>> {
        match call {
            Call::PhalaMq(mq_call) => Some(mq_call),
            _ => None,
        }
    }
}

parameter_types! {
    pub const ExpectedBlockTimeSec: u32 = SECS_PER_BLOCK as u32;
    pub const MinMiningStaking: Balance = 1 * DOLLARS;
    pub const MinContribution: Balance = 1 * CENTS;
    pub const MiningGracePeriod: u64 = 7 * 24 * 3600;
    pub const MinInitP: u32 = 50;
    pub const MiningEnabledByDefault: bool = false;
    pub const MaxPoolWorkers: u32 = 200;
    pub const VerifyPRuntime: bool = true;
    pub const VerifyRelaychainGenesisBlockHash: bool = true;
}

impl pallet_registry::Config for Runtime {
    type Event = Event;
    type AttestationValidator = pallet_registry::IasValidator;
    type UnixTime = Timestamp;
    type VerifyPRuntime = VerifyPRuntime;
    type VerifyRelaychainGenesisBlockHash = VerifyRelaychainGenesisBlockHash;
    type GovernanceOrigin = EnsureRootOrHalfCouncil;
}
impl pallet_mq::Config for Runtime {
    type QueueNotifyConfig = msg_routing::MessageRouteConfig;
    type CallMatcher = MqCallMatcher;
}
impl pallet_mining::Config for Runtime {
    type Event = Event;
    type ExpectedBlockTimeSec = ExpectedBlockTimeSec;
    type MinInitP = MinInitP;
    type Currency = Balances;
    type Randomness = RandomnessCollectiveFlip;
    type OnReward = PhalaStakePool;
    type OnUnbound = PhalaStakePool;
    type OnStopped = PhalaStakePool;
    type OnTreasurySettled = Treasury;
    type UpdateTokenomicOrigin = EnsureRootOrHalfCouncil;
}
impl pallet_stakepool::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type MinContribution = MinContribution;
    type GracePeriod = MiningGracePeriod;
    type MiningEnabledByDefault = MiningEnabledByDefault;
    type MaxPoolWorkers = MaxPoolWorkers;
    type OnSlashed = Treasury;
    type MiningSwitchOrigin = EnsureRootOrHalfCouncil;
    type BackfillOrigin = EnsureRootOrHalfCouncil;
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
    }

    impl pallet_mq_runtime_api::MqApi<Block> for Runtime {
        fn sender_sequence(sender: &phala_types::messaging::MessageOrigin) -> Option<u64> {
            PhalaMq::offchain_ingress(sender)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info()
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade() -> (Weight, Weight) {
            log::info!("try-runtime::on_runtime_upgrade khala.");
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here. If any of the pre/post migration checks fail, we shall stop
            // right here and right now.
            let weight = Executive::try_runtime_upgrade().unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block_no_check(block: Block) -> Weight {
            Executive::execute_block_no_check(block)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            use frame_system_benchmarking::Pallet as SystemBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, pallet_balances, Balances);
            list_benchmark!(list, extra, pallet_bounties, Bounties);
            list_benchmark!(list, extra, pallet_collective, Council);
            list_benchmark!(list, extra, pallet_collective, TechnicalCommittee);
            list_benchmark!(list, extra, pallet_democracy, Democracy);
            // TODO: assertion failed `failed to submit candidacy`
            list_benchmark!(list, extra, pallet_elections_phragmen, PhragmenElection);
            list_benchmark!(list, extra, pallet_identity, Identity);
            list_benchmark!(list, extra, pallet_membership, TechnicalMembership);
            list_benchmark!(list, extra, pallet_multisig, Multisig);
            list_benchmark!(list, extra, pallet_proxy, Proxy);
            list_benchmark!(list, extra, pallet_scheduler, Scheduler);
            list_benchmark!(list, extra, pallet_session, SessionBench::<Runtime>);
            list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
            list_benchmark!(list, extra, pallet_timestamp, Timestamp);
            list_benchmark!(list, extra, pallet_tips, Tips);
            list_benchmark!(list, extra, pallet_treasury, Treasury);
            list_benchmark!(list, extra, pallet_utility, Utility);
            list_benchmark!(list, extra, pallet_vesting, Vesting);
            list_benchmark!(list, extra, pallet_lottery, Lottery);
            // TODO: panic
            list_benchmark!(list, extra, pallet_collator_selection, CollatorSelection);

            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

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

            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_bounties, Bounties);
            add_benchmark!(params, batches, pallet_collective, Council);
            add_benchmark!(params, batches, pallet_collective, TechnicalCommittee);
            add_benchmark!(params, batches, pallet_democracy, Democracy);
            // TODO: assertion failed `failed to submit candidacy`
            add_benchmark!(params, batches, pallet_elections_phragmen, PhragmenElection);
            add_benchmark!(params, batches, pallet_identity, Identity);
            add_benchmark!(params, batches, pallet_membership, TechnicalMembership);
            add_benchmark!(params, batches, pallet_multisig, Multisig);
            add_benchmark!(params, batches, pallet_proxy, Proxy);
            add_benchmark!(params, batches, pallet_scheduler, Scheduler);
            add_benchmark!(params, batches, pallet_session, SessionBench::<Runtime>);
            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);
            add_benchmark!(params, batches, pallet_tips, Tips);
            add_benchmark!(params, batches, pallet_treasury, Treasury);
            add_benchmark!(params, batches, pallet_utility, Utility);
            add_benchmark!(params, batches, pallet_vesting, Vesting);
            add_benchmark!(params, batches, pallet_lottery, Lottery);
            // TODO: panic
            add_benchmark!(params, batches, pallet_collator_selection, CollatorSelection);

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

        inherent_data.check_extrinsics(&block)
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
    CheckInherents = CheckInherents,
}
