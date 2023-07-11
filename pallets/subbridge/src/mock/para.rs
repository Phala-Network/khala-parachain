use super::ParachainXcmRouter;
use crate::{
	chainbridge, dynamic_trader::DynamicWeightTrader, fungible_adapter::XTransferAdapter, helper,
	xcmbridge, xtransfer,
};

use assets_registry;
use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU32, Everything, Nothing},
	weights::Weight,
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};

use cumulus_primitives_core::{ChannelStatus, GetChannelInfo, ParaId};
use phala_pallet_common::WrapSlice;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::{prelude::*, Weight as XCMWeight};
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, MintLocation, NativeAsset, NoChecking,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{traits::WithOriginFilter, Config, XcmExecutor};

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

		// Parachain staff
		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},

		// Local palelts
		AssetsRegistry: assets_registry::{Pallet, Call, Storage, Event<T>},
		XcmBridge: xcmbridge::{Pallet, Storage, Event<T>},
		ChainBridge: chainbridge::{Pallet, Call, Storage, Event<T>},
		XTransfer: xtransfer::{Pallet, Call, Storage, Event<T>},
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
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = AssetDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ConstU32<1>;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1; // 1 Unit deposit to create asset
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<5>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl pallet_parachain_info::Config for Runtime {}

parameter_types! {
	pub const TestChainId: u8 = 5;
	pub const ResourceIdGenerationSalt: Option<u128> = Some(5);
	pub const ProposalLifetime: u64 = 100;
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub UniversalLocation: InteriorMultiLocation =
		X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();

	pub TREASURY: AccountId32 = AccountId32::new([4u8; 32]);
		// We define two test assets to simulate tranfer assets to reserve location and unreserve location,
	// we must defiend here because those need be configed as fee payment assets
	pub SoloChain0AssetLocation: MultiLocation = MultiLocation::new(
		1,
		X4(
			Parachain(2004),
			WrapSlice(assets_registry::CB_ASSET_KEY).into_generalkey(),
			GeneralIndex(0),
			WrapSlice(b"an asset").into_generalkey(),
		),
	);
	pub SoloChain2AssetLocation: MultiLocation = MultiLocation::new(
		1,
		X4(
			Parachain(2004),
			WrapSlice(assets_registry::CB_ASSET_KEY).into_generalkey(),
			GeneralIndex(2),
			WrapSlice(b"an asset").into_generalkey(),
		),
	);
	pub NativeExecutionPrice: u128 = 1;
	pub WeightPerSecond: u64 = 1;
	pub NativeAssetLocation: MultiLocation = MultiLocation::here();
	pub NativeAssetSygmaResourceId: [u8; 32] = [0; 32];
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);
parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: XCMWeight = 1u64.into();
	pub const MaxInstructions: u32 = 100;
	pub ParaTreasuryAccount: AccountId = PalletId(*b"py/trsry").into_account_truncating();
	pub CheckingAccountForCurrencyAdapter: Option<(AccountId, MintLocation)> = None;
	pub CheckingAccountForFungibleAdapter: AccountId = PalletId(*b"checking").into_account_truncating();
}
match_types! {
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

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
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
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = DynamicWeightTrader<
		WeightPerSecond,
		<Runtime as pallet_assets::Config>::AssetId,
		AssetsRegistry,
		helper::XTransferTakeRevenue<Self::AssetTransactor, AccountId, ParaTreasuryAccount>,
	>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = WithOriginFilter<Everything>;
	type SafeCallFilter = Everything;
}
parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(10);
	pub const BridgeEventLimit: u32 = 1024;
}
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub type XcmRouter = ParachainXcmRouter<ParachainInfo>;

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ChannelInfo;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = ();
}
impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
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

impl assets_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RegistryCommitteeOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type MinBalance = AssetDeposit;
	type NativeExecutionPrice = NativeExecutionPrice;
	type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
	type ReserveAssetChecker = assets_registry::ReserveAssetFilter<
		ParachainInfo,
		assets_registry::NativeAssetFilter<ParachainInfo>,
	>;
	type ResourceIdGenerationSalt = ResourceIdGenerationSalt;
	type NativeAssetLocation = NativeAssetLocation;
	type NativeAssetSygmaResourceId = NativeAssetSygmaResourceId;
}

impl chainbridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = EnsureRoot<Self::AccountId>;
	type Proposal = RuntimeCall;
	type BridgeChainId = TestChainId;
	type Currency = Balances;
	type ProposalLifetime = ProposalLifetime;
	type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
	type NativeExecutionPrice = NativeExecutionPrice;
	type TreasuryAccount = TREASURY;
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
