use super::ParachainXcmRouter;
use crate::{pallet_xcm_transfer, pallet_xtransfer_assets, xtransfer_matcher};

use frame_support::{
	construct_runtime, match_type, parameter_types,
	traits::Everything,
	weights::{constants::WEIGHT_PER_SECOND, IdentityFee, Weight},
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32,
};

use cumulus_primitives_core::{ChannelStatus, GetChannelInfo, ParaId};
use polkadot_parachain::primitives::Sibling;
use xcm::v1::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, EnsureXcmOrigin,
	FixedWeightBounds, LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsDefault,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

pub use parachains_common::Index;
pub use parachains_common::*;

pub(crate) type Balance = u128;
pub const DOLLARS: Balance = 1_000_000_000_000;
pub const CENTS: Balance = DOLLARS / 100;

pub type AccountId = AccountId32;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
pub(crate) type BlockNumber = u64;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

		// Parachain staff
		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},

		// local palelts
		XcmTransfer: pallet_xcm_transfer::{Pallet, Call, Event<T>, Storage},
		XTransferAssets: pallet_xtransfer_assets::{Pallet, Call, Event<T>, Storage},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 20;
	pub const MinimumPeriod: u64 = 1;
	pub const ExpectedBlockTimeSec: u32 = 12;
	pub const MinMiningStaking: Balance = 1 * DOLLARS;
	pub const MinContribution: Balance = 1 * CENTS;
	pub const MiningGracePeriod: u64 = 7 * 24 * 3600;
}
impl system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ();
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl pallet_parachain_info::Config for Runtime {}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	pub const LocalLocation: MultiLocation = Here.into();
	pub const DOTMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: Here };
	pub const ParaAMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(1)) };
	pub const ParaBMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(2)) };
	pub const ParaCMultiAssetId: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(3)) };
}

pub type LocationToAccountId = (
	ParentIsDefault<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	RelayChainAsNative<RelayChainOrigin, Origin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	SignedAccountId32AsNative<RelayNetwork, Origin>,
);
parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1;
	// pub const MaxInstructions: u32 = 100;
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
	// ^^^ Parent and its exec plurality get free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = XTransferAssets;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type Trader = (
		UsingComponents<IdentityFee<Balance>, DOTMultiAssetId, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaAMultiAssetId, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaBMultiAssetId, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaCMultiAssetId, AccountId, Balances, ()>,
	);
	type ResponseHandler = ();
	type SubscriptionService = ();
}
parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

pub type XcmRouter = ParachainXcmRouter<ParachainInfo>;

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
pub struct ChannelInfo;
impl GetChannelInfo for ChannelInfo {
	fn get_channel_status(_id: ParaId) -> ChannelStatus {
		ChannelStatus::Ready(10, 10)
	}
	fn get_channel_max(_id: ParaId) -> Option<usize> {
		Some(usize::max_value())
	}
}
impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ChannelInfo;
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
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type LocationInverter = LocationInverter<Ancestry>;
}

impl pallet_xtransfer_assets::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type XTransferCommitteeOrigin = EnsureRoot<AccountId>;
	type FungibleMatcher = xtransfer_matcher::IsSiblingParachainsConcrete<XTransferAssets>;
	type AccountIdConverter = LocationToAccountId;
	type ParachainInfo = ParachainInfo;
}
