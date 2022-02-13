use super::ParachainXcmRouter;
use crate::bridge::pallet::{BridgeChainId, BridgeTransact, ResourceId};
use crate::{pallet_assets_wrapper, pallet_xcm_transfer, xcm_helper};

use frame_support::{
	construct_runtime, match_type,
	pallet_prelude::*,
	parameter_types,
	traits::{ConstU128, ConstU32, Contains, Everything},
	weights::{IdentityFee, Weight},
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_core::{H256, U256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32,
};

use cumulus_primitives_core::{ChannelStatus, GetChannelInfo, ParaId};
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, LocationInverter, NativeAsset,
	ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
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

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		AssetsWrapper: pallet_assets_wrapper::{Pallet, Call, Storage, Event<T>},

		// Parachain staff
		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},

		// Local palelts
		XcmTransfer: pallet_xcm_transfer::{Pallet, Call, Event<T>, Storage},
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
	type MaxConsumers = ConstU32<2>;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = AssetDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const AssetDeposit: Balance = 1; // 1 Unit deposit to create asset
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AssetId = u32;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl pallet_parachain_info::Config for Runtime {}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();

	pub KSMLocation: MultiLocation = MultiLocation::new(1, Here);
	pub LocalParaLocation: MultiLocation = MultiLocation::new(0, Here);
	pub ParaALocation: MultiLocation = MultiLocation::new(1, X1(Parachain(1)));
	pub ParaBLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(2)));
	pub ParaCLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(3)));

	pub KSMAssetId: AssetId = KSMLocation::get().into();
	pub ParaAAssetId: AssetId = ParaALocation::get().into();
	pub ParaBAssetId: AssetId = ParaBLocation::get().into();
	pub ParaCAssetId: AssetId = ParaCLocation::get().into();

	pub FeeAssets: MultiAssets = [
		KSMAssetId::get().into_multiasset(Fungibility::Fungible(u128::MAX)),
		ParaAAssetId::get().into_multiasset(Fungibility::Fungible(u128::MAX)),
		ParaBAssetId::get().into_multiasset(Fungibility::Fungible(u128::MAX)),
		ParaCAssetId::get().into_multiasset(Fungibility::Fungible(u128::MAX)),
	].to_vec().into();

	pub const DefaultDestChainXcmFee: Balance = 10;
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
	AllowUnpaidExecutionFrom<Everything>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	xcm_helper::NativeAssetMatcher<xcm_helper::NativeAssetFilter<ParachainInfo>>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

pub struct AssetChecker;
impl Contains<u32> for AssetChecker {
	fn contains(_: &u32) -> bool {
		false
	}
}

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	xcm_helper::ConcreteAssetsMatcher<
		<Runtime as pallet_assets::Config>::AssetId,
		Balance,
		AssetsWrapper,
	>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We do not support teleport assets
	AssetChecker,
	// We do not support teleport assets
	(),
>;

impl BridgeTransact for () {
	fn transfer_fungible(
		_dest_id: BridgeChainId,
		_resource_id: ResourceId,
		_to: Vec<u8>,
		_amount: U256,
	) -> DispatchResult {
		Ok(())
	}

	fn transfer_nonfungible(
		_dest_id: BridgeChainId,
		_resource_id: ResourceId,
		_token_id: Vec<u8>,
		_to: Vec<u8>,
		_metadata: Vec<u8>,
	) -> DispatchResult {
		Ok(())
	}

	fn transfer_generic(
		_dest_id: BridgeChainId,
		_resource_id: ResourceId,
		_metadata: Vec<u8>,
	) -> DispatchResult {
		Ok(())
	}

	fn reservation_account() -> [u8; 32] {
		[0; 32]
	}
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = xcm_helper::XTransferAdapter<
		CurrencyTransactor,
		FungiblesTransactor,
		XcmTransfer,
		XcmTransfer,
		xcm_helper::NativeAssetFilter<ParachainInfo>,
		(),
		(),
		AccountId,
		(),
	>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = (
		UsingComponents<IdentityFee<Balance>, KSMLocation, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, LocalParaLocation, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaALocation, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaBLocation, AccountId, Balances, ()>,
		UsingComponents<IdentityFee<Balance>, ParaCLocation, AccountId, Balances, ()>,
	);
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
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
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
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
	type NativeAssetChecker = xcm_helper::NativeAssetFilter<ParachainInfo>;
	type FeeAssets = FeeAssets;
	type DefaultFee = DefaultDestChainXcmFee;
}

impl pallet_assets_wrapper::Config for Runtime {
	type Event = Event;
	type AssetsCommitteeOrigin = EnsureRoot<AccountId>;
	type MinBalance = AssetDeposit;
}
