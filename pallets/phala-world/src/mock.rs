use crate::{pallet_pw_incubation, pallet_pw_marketplace, pallet_pw_nft_sale};

use frame_support::{
	construct_runtime, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU32, Everything, GenesisBuild, OnFinalize},
	weights::Weight,
};
use frame_system::EnsureRoot;
use sp_core::{crypto::AccountId32, H256};

use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};

type AccountId = AccountId32;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type BlockNumber = u64;
pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 1_000;
pub const INIT_TIMESTAMP_SECONDS: u64 = 30;
pub const BLOCK_TIME_SECONDS: u64 = 1;
// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Uniques: pallet_uniques::{Pallet, Storage, Event<T>},
		RmrkCore: pallet_rmrk_core::{Pallet, Call, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		RmrkMarket: pallet_rmrk_market::{Pallet, Call, Event<T>},
		PWNftSale: pallet_pw_nft_sale::{Pallet, Call, Storage, Event<T>},
		PWIncubation: pallet_pw_incubation::{Pallet, Call, Storage, Event<T>},
		PWMarketplace: pallet_pw_marketplace::{Pallet, Call, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 0);
	pub const MaximumBlockLength: u32 = 2 * 1024;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = BlockNumber;
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
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<2>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ConstU32<1>;
}

parameter_types! {
	pub ClassBondAmount: Balance = 100;
	pub MaxMetadataLength: u32 = 256;
	pub const ResourceSymbolLimit: u32 = 10;
	pub const PartsLimit: u32 = 25;
	pub const MaxPriorities: u32 = 25;
	pub const PropertiesLimit: u32 = 15;
	pub const CollectionSymbolLimit: u32 = 100;
	pub const MaxResourcesOnMint: u32 = 100;
	pub const NestingBudget: u32 = 20;
}

impl pallet_rmrk_core::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ProtocolOrigin = EnsureRoot<AccountId>;
	type ResourceSymbolLimit = ResourceSymbolLimit;
	type PartsLimit = PartsLimit;
	type MaxPriorities = MaxPriorities;
	type PropertiesLimit = PropertiesLimit;
	type NestingBudget = NestingBudget;
	type CollectionSymbolLimit = CollectionSymbolLimit;
	type MaxResourcesOnMint = MaxResourcesOnMint;
	type WeightInfo = pallet_rmrk_core::weights::SubstrateWeight<Test>;
	type TransferHooks = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = pallet_rmrk_core::RmrkBenchmark;
}

parameter_types! {
	pub const CollectionDeposit: Balance = 10_000 * PHA; // 1 UNIT deposit to create collection
	pub const ItemDeposit: Balance = 100 * PHA; // 1/100 UNIT deposit to create item
	pub const StringLimit: u32 = 50;
	pub const KeyLimit: u32 = 32; // Max 32 bytes per key
	pub const ValueLimit: u32 = 256; // Max 64 bytes per value
	pub const UniquesMetadataDepositBase: Balance = 1000 * PHA;
	pub const ZeroDeposit: Balance = 0 * PHA;
	pub const AttributeDepositBase: Balance = 100 * PHA;
	pub const DepositPerByte: Balance = 10 * PHA;
}

impl pallet_uniques::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type Locker = pallet_rmrk_core::Pallet<Test>;
	type CollectionDeposit = ZeroDeposit;
	type ItemDeposit = ZeroDeposit;
	type MetadataDepositBase = ZeroDeposit;
	type AttributeDepositBase = ZeroDeposit;
	type DepositPerByte = ZeroDeposit;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

parameter_types! {
	pub const MinimumOfferAmount: Balance = 50 * UNITS;
	pub const MarketFee: Permill = Permill::from_parts(5_000);
}

impl pallet_rmrk_market::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ProtocolOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type MinimumOfferAmount = MinimumOfferAmount;
	type WeightInfo = pallet_rmrk_market::weights::SubstrateWeight<Test>;
	type MarketplaceHooks = PWMarketplace;
	type MarketFee = MarketFee;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const SecondsPerEra: u64 = 5 * BLOCK_TIME_SECONDS;
	pub const MinBalanceToClaimSpirit: Balance = 10 * PHA;
	pub const LegendaryOriginOfShellPrice: Balance = 1_000_000 * PHA;
	pub const MagicOriginOfShellPrice: Balance = 1_000 * PHA;
	pub const PrimeOriginOfShellPrice: Balance = 10 * PHA;
	pub const MaxMintPerRace: u32 = 2;
	pub const IterLimit: u32 = 200;
	pub const FoodPerEra: u32 = 5;
	pub const MaxFoodFeedSelf: u8 = 2;
	pub const IncubationDurationSec: u64 = 600;
}

impl pallet_pw_nft_sale::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type Time = pallet_timestamp::Pallet<Test>;
	type SecondsPerEra = SecondsPerEra;
	type MinBalanceToClaimSpirit = MinBalanceToClaimSpirit;
	type LegendaryOriginOfShellPrice = LegendaryOriginOfShellPrice;
	type MagicOriginOfShellPrice = MagicOriginOfShellPrice;
	type PrimeOriginOfShellPrice = PrimeOriginOfShellPrice;
	type IterLimit = IterLimit;
}

impl pallet_pw_incubation::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type FoodPerEra = FoodPerEra;
	type MaxFoodFeedSelf = MaxFoodFeedSelf;
	type IncubationDurationSec = IncubationDurationSec;
}

impl pallet_pw_marketplace::Config for Test {
	type RuntimeEvent = RuntimeEvent;
}

pub fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
		System::on_finalize(System::block_number());
		PWNftSale::on_finalize(System::block_number());
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
	}
}
// overlord_pair = sr25519::Pair::from_seed(b"28133080042813308004281330800428");
pub const OVERLORD: AccountId = AccountId::new([
	176, 155, 174, 174, 163, 79, 183, 121, 13, 202, 60, 83, 242, 187, 181, 64, 51, 220, 13, 104,
	162, 108, 19, 241, 150, 65, 49, 48, 136, 28, 19, 101,
]);
pub const PAYEE: AccountId = AccountId::new([
	17, 15, 74, 17, 63, 79, 183, 121, 13, 202, 60, 83, 242, 187, 181, 64, 51, 22, 13, 104, 162,
	108, 19, 41, 150, 65, 49, 48, 136, 28, 19, 10,
]);
pub const SIGNER: AccountId = AccountId::new([
	182, 208, 215, 197, 76, 223, 247, 105, 170, 67, 40, 36, 72, 233, 195, 237, 36, 116, 177, 66,
	63, 77, 158, 35, 19, 31, 193, 190, 71, 206, 191, 113,
]);
// alice_pair = sr25519::Pair::from_seed(b"12345678901234567890123456789012");
pub const ALICE: AccountId = AccountId::new([
	116, 28, 8, 160, 111, 65, 197, 150, 96, 143, 103, 116, 37, 155, 217, 4, 51, 4, 173, 250, 93,
	62, 234, 98, 118, 11, 217, 190, 151, 99, 77, 99,
]);
// bob_pair = sr25519::Pair::from_seed(b"09876543210987654321098765432109");
pub const BOB: AccountId = AccountId::new([
	250, 140, 153, 155, 88, 13, 83, 23, 193, 161, 236, 241, 58, 213, 107, 213, 230, 33, 38, 154,
	78, 125, 67, 186, 54, 157, 62, 131, 179, 150, 232, 82,
]);
// charlie_pair = sr25519::Pair::from_seed(b"19004878537190048785371900487853");
pub const CHARLIE: AccountId = AccountId::new([
	144, 178, 175, 207, 158, 226, 236, 9, 193, 197, 35, 61, 203, 142, 237, 60, 100, 189, 217, 163,
	184, 20, 116, 158, 252, 151, 72, 114, 185, 129, 78, 43,
]);

pub const PHA: Balance = 1;
pub const UNITS: Balance = 100_000_000_000;
// Time is measured by number of blocks.
pub const INCUBATION_DURATION_SEC: u64 = 600;

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}
// Build genesis storage according to the mock runtime.
impl ExtBuilder {
	pub fn build(self, overlord_key: AccountId32) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		pallet_pw_nft_sale::GenesisConfig::<Test> {
			zero_day: None,
			overlord: Some(overlord_key),
			era: 0,
			can_claim_spirits: false,
			can_purchase_rare_origin_of_shells: false,
			can_purchase_prime_origin_of_shells: false,
			can_preorder_origin_of_shells: false,
			last_day_of_sale: false,
			spirit_collection_id: None,
			origin_of_shell_collection_id: None,
			is_origin_of_shells_inventory_set: false,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(OVERLORD, 2_813_308_004 * PHA),
				(ALICE, 20_000_000 * PHA),
				(BOB, 15_000 * PHA),
				(CHARLIE, 150_000 * PHA),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::set_block_number(1);
			Timestamp::set_timestamp(INIT_TIMESTAMP);
		});
		ext
	}
}
