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

//! Khala runtime.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

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

// Constant values used within the runtime.
pub mod constants;
use constants::currency::*;

use codec::{Decode, Encode};
use sp_api::impl_runtime_apis;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4, _5},
    OpaqueMetadata,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{AccountIdLookup, Block as BlockT, ConvertInto},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, match_type, parameter_types,
    traits::{
        All, Currency, Imbalance, InstanceFilter, IsInVec, KeyOwnerProofSystem, LockIdentifier,
        OnUnbalanced, Randomness, U128CurrencyToVote,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    PalletId, RuntimeDebug, StorageValue,
};
pub use frame_system::Call as SystemCall;
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureOneOf, EnsureRoot,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
pub use runtime_common::*;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot imports
use polkadot_pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{BodyId, Junction, MultiAsset, MultiLocation, NetworkId, Xcm};
use xcm_builder::{
    AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
    EnsureXcmOrigin, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset,
    ParentAsSuperuser, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    // Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;

    pub type SessionHandlers = ();

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
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
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
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, Hasher>;
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
        // System support stuff;
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 1,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage} = 3,
        ParachainInfo: cumulus_pallet_parachain_info::{Pallet, Storage, Config} = 4,
        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>} = 5,

        // Monetary stuff;
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 11,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 12,

        // Collator support.
        Aura: pallet_aura::{Pallet, Config<T>} = 21,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Config} = 22,

        // The main stage.
        Utility: pallet_utility::{Pallet, Call, Event} = 31,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 32,
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 33,
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 34,
        Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 35,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 36,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 37,

        Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 51,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 52,
        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 53,
        Lottery: pallet_lottery::{Pallet, Call, Storage, Event<T>} = 54,
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config, Event<T>} = 55,
        TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 56,
        TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 57,

        // XCM helpers.
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 81,
        PolkadotXcm: polkadot_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 82,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 83,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 84,

        Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>} = 99,
    }
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
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
    pub const SS58Prefix: u8 = 30;
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
    type Header = generic::Header<BlockNumber, Hasher>;
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
    type BaseCallFilter = ();
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
}

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
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub enum ProxyType {
    /// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
    Any,
    /// Can execute any call that does not transfer funds, including asset transfers.
    NonTransfer,
    /// Proxy with the ability to reject time-delay proxy announcements.
    CancelProxy,
    /// Governance
    Governance,
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
            ProxyType::NonTransfer => !matches!(
                c,
                Call::Balances(..) | Call::Vesting(pallet_vesting::Call::vested_transfer(..))
            ),
            ProxyType::CancelProxy => matches!(
                c,
                Call::Proxy(pallet_proxy::Call::reject_announcement(..))
                    | Call::Utility(..)
                    | Call::Multisig(..)
            ),
            ProxyType::Governance => matches!(
                c,
                Call::Democracy(..)
                    | Call::Council(..)
                    | Call::TechnicalCommittee(..)
                    | Call::Treasury(..)
                    | Call::Lottery(..)
                    | Call::Bounties(..)
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
    type ScheduleOrigin = EnsureRoot<AccountId>;
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
    pub const ExistentialDeposit: Balance = 1 * DOLLARS;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    // used for benchmark
    pub const BenchmarkingExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLICENTS;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: pallet_transaction_payment::Multiplier =
        pallet_transaction_payment::Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 80% to treasury, 20% to author
            let mut split = fees.ration(80, 20);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 80% to treasury, 20% to author (though this can be anything)
                tips.ration_merge_into(80, 20, &mut split);
            }
            Treasury::on_unbalanced(split.0);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = pallet_transaction_payment::TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
    >;
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
    pub const ConfigDepositBase: Balance = 5 * DOLLARS;
    pub const FriendDepositFactor: Balance = 50 * CENTS;
    pub const MaxFriends: u16 = 9;
    pub const RecoveryDeposit: Balance = 5 * DOLLARS;
}

impl pallet_recovery::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type ConfigDepositBase = ConfigDepositBase;
    type FriendDepositFactor = FriendDepositFactor;
    type MaxFriends = MaxFriends;
    type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 100 * DOLLARS;
}

impl pallet_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
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
    type ManagerOrigin = EnsureRoot<AccountId>;
    type MaxCalls = MaxCalls;
    type ValidateCall = Lottery;
    type MaxGenerateRandom = MaxGenerateRandom;
    type WeightInfo = pallet_lottery::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type Call = Call;
    type Event = Event;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = cumulus_pallet_parachain_info::Pallet<Runtime>;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl cumulus_pallet_parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const RelayChainLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
    pub const RelayChainNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Junction::Parachain(ParachainInfo::parachain_id().into()).into();
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
    AccountId32Aliases<RelayChainNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<RelayChainLocation>,
    // Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports.
    (),
>;

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
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<Origin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayChainNetwork, Origin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<Origin>,
);

parameter_types! {
    pub UnitWeightCost: Weight = 1_000;
}

match_type! {
    pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
        MultiLocation::X1(Junction::Parent) |
        MultiLocation::X2(Junction::Parent, Junction::Plurality { id: BodyId::Unit, .. })
    };
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<All<MultiLocation>>,
    AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
    // ^^^ Parent & its unit plurality gets free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation of PHA
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
    type Trader =
        UsingComponents<IdentityFee<Balance>, RelayChainLocation, AccountId, Balances, ()>;
    type ResponseHandler = (); // Don't handle responses for now.
}

parameter_types! {
    pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, RelayChainNetwork>,);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl polkadot_pallet_xcm::Config for Runtime {
    type Event = Event;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = All<(MultiLocation, Vec<MultiAsset>)>;
    type XcmReserveTransferFilter = All<(MultiLocation, Vec<MultiAsset>)>;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
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
    pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
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
    pub const Burn: Permill = Permill::from_percent(50);
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
    pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
    pub const InstantAllowed: bool = true;
    pub const MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
    pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
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
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin =
        pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin =
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>;
    type InstantOrigin =
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin =
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureOneOf<
        AccountId,
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
    >;
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

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
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
            Runtime::metadata().into()
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
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
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

        #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

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
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_multisig, Multisig);
            add_benchmark!(params, batches, pallet_proxy, Proxy);
            add_benchmark!(params, batches, pallet_utility, Utility);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block!(
    Runtime,
    cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
);
