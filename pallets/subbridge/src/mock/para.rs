use super::ParachainXcmRouter;
use crate::{
	chainbridge, dynamic_trader::DynamicWeightTrader, fungible_adapter::XTransferAdapter, helper,
	pbridge, xcmbridge, xtransfer,
};

use assets_registry;
use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{ConstU128, ConstU32, Contains, Everything},
	weights::Weight,
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	AccountId32,
};

use cumulus_primitives_core::{ChannelStatus, GetChannelInfo, ParaId};
use phala_pallets::{
	attestation::{Attestation, AttestationValidator, Error as AttestationError, IasFields},
	pallet_mq, pallet_registry,
};
use phala_types::contract::{ContractClusterId, ContractId};
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, LocationInverter, NativeAsset,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
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
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},

		// Parachain staff
		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},

		// Local palelts
		PhalaMq: pallet_mq::{Pallet, Call},
		PhalaRegistry: pallet_registry::{Pallet, Event<T>, Storage, Config<T>},
		AssetsRegistry: assets_registry::{Pallet, Call, Storage, Event<T>},
		XcmBridge: xcmbridge::{Pallet, Storage, Event<T>},
		PBridge: pbridge::{Pallet, Call, Storage, Event<T>},
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

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
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
	pub const TestChainId: u8 = 5;
	pub const ResourceIdGenerationSalt: Option<u128> = Some(5);
	pub const ProposalLifetime: u64 = 100;
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();

	pub TREASURY: AccountId32 = AccountId32::new([4u8; 32]);
		// We define two test assets to simulate tranfer assets to reserve location and unreserve location,
	// we must defiend here because those need be configed as fee payment assets
	pub SoloChain0AssetLocation: MultiLocation = MultiLocation::new(
		1,
		X4(
			Parachain(2004),
			GeneralKey(assets_registry::CB_ASSET_KEY.to_vec().try_into().expect("less than length limit; qed")),
			GeneralIndex(0),
			GeneralKey(b"an asset".to_vec().try_into().expect("less than length limit; qed")),
		),
	);
	pub SoloChain2AssetLocation: MultiLocation = MultiLocation::new(
		1,
		X4(
			Parachain(2004),
			GeneralKey(assets_registry::CB_ASSET_KEY.to_vec().try_into().expect("less than length limit; qed")),
			GeneralIndex(2),
			GeneralKey(b"an asset".to_vec().try_into().expect("less than length limit; qed")),
		),
	);
	pub NativeExecutionPrice: u128 = 1;
	pub WeightPerSecond: u64 = 1;
	pub PBridgeSelector: [u8; 4] = [1, 1, 1, 1];
	pub const VerifyPRuntime: bool = false;
	pub const VerifyRelaychainGenesisBlockHash: bool = true;
	// Dummy contract id of PHA in fat contract
	pub TEST_PHA_CONTRACT_ID: ContractId = [0; 32].into();
	// Dummy cluster id of relevant PHA contract
	pub TEST_PHA_CLUSTER_ID: ContractClusterId = [0; 32].into();
	// Dummy SubBridge contract address deployed in relevant cluster
	pub TEST_SUBBRIDGE_CONTRACT_ID: ContractId = [255; 32].into();
	pub TEST_SUBBRIDGE_CLUSTER_ID: ContractClusterId = [0; 32].into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
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
	pub ParaTreasuryAccount: AccountId = PalletId(*b"py/trsry").into_account_truncating();
	pub ParaCheckingAccount: AccountId = PalletId(*b"py/check").into_account_truncating();
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
	ParaCheckingAccount,
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
	AssetChecker,
	// We do not support teleport assets
	ParaCheckingAccount,
>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
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
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = DynamicWeightTrader<
		WeightPerSecond,
		<Runtime as pallet_assets::Config>::AssetId,
		AssetsRegistry,
		helper::XTransferTakeRevenue<Self::AssetTransactor, AccountId, TREASURY>,
	>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
}
parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
	pub const BridgeEventLimit: u32 = 1024;
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
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
}
impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

impl xcmbridge::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
	type AssetsRegistry = AssetsRegistry;
}

impl assets_registry::Config for Runtime {
	type Event = Event;
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
}

impl chainbridge::Config for Runtime {
	type Event = Event;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Proposal = Call;
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

pub struct MockValidator;
impl AttestationValidator for MockValidator {
	fn validate(
		_attestation: &Attestation,
		_user_data_hash: &[u8; 32],
		_now: u64,
		_verify_pruntime: bool,
		_pruntime_allowlist: Vec<Vec<u8>>,
	) -> Result<IasFields, AttestationError> {
		Ok(IasFields {
			mr_enclave: [0u8; 32],
			mr_signer: [0u8; 32],
			isv_prod_id: [0u8; 2],
			isv_svn: [0u8; 2],
			report_data: [0u8; 64],
			confidence_level: 128u8,
		})
	}
}

impl pallet_registry::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type AttestationValidator = MockValidator;
	type UnixTime = Timestamp;
	type VerifyPRuntime = VerifyPRuntime;
	type VerifyRelaychainGenesisBlockHash = VerifyRelaychainGenesisBlockHash;
	type GovernanceOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

impl pallet_mq::Config for Runtime {
	type QueueNotifyConfig = ();
	type CallMatcher = MqCallMatcher;
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

impl pbridge::Config for Runtime {
	type Event = Event;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Currency = Balances;
	type NativeAssetChecker = assets_registry::NativeAssetFilter<ParachainInfo>;
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
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type ContractSelector = PBridgeSelector;
	type NativeAssetContractId = TEST_PHA_CONTRACT_ID;
}

impl xtransfer::Config for Runtime {
	type Event = Event;
	type Bridge = (
		xcmbridge::BridgeTransactImpl<Runtime>,
		pbridge::BridgeTransactImpl<Runtime>,
		chainbridge::BridgeTransactImpl<Runtime>,
	);
}
