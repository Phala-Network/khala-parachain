#![cfg(test)]
use crate as pallet_index;
use frame_support::{
	pallet_prelude::ConstU32,
	parameter_types,
	sp_runtime::{
		testing::H256,
		traits::{AccountIdConversion, BlakeTwo256, CheckedConversion, IdentityLookup},
		AccountId32, Perbill,
	},
	traits::{AsEnsureOriginWithArg, ConstU128, Contains},
	PalletId,
};
use frame_system::{self as system, EnsureRoot, EnsureSigned};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::BuildStorage;
use sp_std::{marker::PhantomData, result};
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, FungiblesAdapter, MintLocation, NoChecking,
	ParentIsPreset, SiblingParachainConvertsVia,
};
use xcm_executor::traits::{Error as MatchError, MatchesFungible, MatchesFungibles};

type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type Balance = u128;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const ENDOWED_BALANCE: Balance = 100_000_000;

frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		PalletIndex: pallet_index::{Pallet, Call, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		AssetsRegistry: assets_registry::{Pallet, Call, Storage, Event<T>},
		ParachainInfo: pallet_parachain_info::{Pallet, Storage, Config<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const MaxLocks: u32 = 100;
	pub const MinimumPeriod: u64 = 1;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<2>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = ();
}

parameter_types! {
	pub const AssetDeposit: Balance = 1; // 1 Unit deposit to create asset
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

pub type AssetId = u32;
impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
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
	pub NativeExecutionPrice: u128 = 1;
	pub ResourceIdGenerationSalt: Option<u128> = Some(3);
	pub NativeAssetLocation: MultiLocation = MultiLocation::here();
	pub NativeAssetSygmaResourceId: [u8; 32] = [0; 32];
}
impl assets_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RegistryCommitteeOrigin = EnsureRoot<Self::AccountId>;
	type Currency = Balances;
	type MinBalance = ExistentialDeposit;
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
impl pallet_parachain_info::Config for Test {}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub CheckingAccountForCurrencyAdapter: Option<(AccountId32, MintLocation)> = None;
	pub CheckingAccountForFungibleAdapter: AccountId32 = PalletId(*b"checking").into_account_truncating();
	pub TestAssetLocation: MultiLocation = MultiLocation::new(1, X1(GeneralIndex(123)));
	pub TestAssetAssetId: AssetId = 0;
}

pub struct SimpleNativeAssetMatcher;
impl<B: TryFrom<u128>> MatchesFungible<B> for SimpleNativeAssetMatcher {
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		match (&a.id, &a.fun) {
			(Concrete(location), Fungible(ref amount)) => {
				if location == &MultiLocation::here() {
					CheckedConversion::checked_from(*amount)
				} else {
					None
				}
			}
			_ => None,
		}
	}
}

pub struct SimpleForeignAssetMatcher(PhantomData<()>);
impl MatchesFungibles<AssetId, Balance> for SimpleForeignAssetMatcher {
	fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), MatchError> {
		match (&a.fun, &a.id) {
			(Fungible(ref amount), Concrete(ref id)) => {
				if id == &TestAssetLocation::get() {
					Ok((TestAssetAssetId::get(), *amount))
				} else {
					Err(MatchError::AssetNotHandled)
				}
			}
			_ => Err(MatchError::AssetNotHandled),
		}
	}
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId32>,
	SiblingParachainConvertsVia<Sibling, AccountId32>,
	AccountId32Aliases<RelayNetwork, AccountId32>,
);

pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	SimpleNativeAssetMatcher,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// We don't track any teleports of `Balances`.
	CheckingAccountForCurrencyAdapter,
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
	SimpleForeignAssetMatcher,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// We do not support teleport assets
	NoChecking,
	// We do not support teleport assets
	CheckingAccountForFungibleAdapter,
>;

parameter_types! {
	pub const FeeReserveAccount: AccountId32 = AccountId32::new([3u8; 32]);
}

pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);
impl pallet_index::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CommitteeOrigin = EnsureRoot<Self::AccountId>;
	type FeeReserveAccount = FeeReserveAccount;
	type AssetTransactor = AssetTransactors;
	type AssetsRegistry = AssetsRegistry;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();

	pallet_parachain_info::GenesisConfig::<Test> {
		_mark: Default::default(),
		parachain_id: 2004u32.into(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let assets_registry_account = assets_registry::ASSETS_REGISTRY_ID.into_account_truncating();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, ENDOWED_BALANCE),
			(BOB, ENDOWED_BALANCE),
			(assets_registry_account, ENDOWED_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
	let mut actual: Vec<RuntimeEvent> = system::Pallet::<Test>::events()
		.iter()
		.map(|e| e.event.clone())
		.collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("event expected");
		assert_eq!(next, evt, "Events don't match");
	}
}
