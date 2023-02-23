#![cfg(test)]

use frame_support::{
	pallet_prelude::ConstU32,
	parameter_types,
	sp_runtime::{
		testing::{Header, H256},
		traits::{AccountIdConversion, BlakeTwo256, CheckedConversion, IdentityLookup},
		AccountId32, Perbill,
	},
	PalletId,
};
use frame_system::{self as system, EnsureRoot};

use crate as pallet_index;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, ParentIsPreset, SiblingParachainConvertsVia,
};
use xcm_executor::traits::MatchesFungible;

pub(crate) type Balance = u128;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const ENDOWED_BALANCE: Balance = 100_000_000;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		PalletIndex: pallet_index::{Pallet, Call, Storage, Event<T>},
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
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
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
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub ParaCheckingAccount: AccountId32 = PalletId(*b"py/check").into_account_truncating();
}

pub struct NativeAssetMatcher;
impl<B: TryFrom<u128>> MatchesFungible<B> for NativeAssetMatcher {
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		match (&a.id, &a.fun) {
			(Concrete(location), Fungible(ref amount)) if location == &MultiLocation::here() => {
				CheckedConversion::checked_from(*amount)
			}
			_ => None,
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
	NativeAssetMatcher,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId32,
	// We don't track any teleports of `Balances`.
	ParaCheckingAccount,
>;

impl pallet_index::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CommitteeOrigin = EnsureRoot<Self::AccountId>;
	type AssetTransactor = CurrencyTransactor;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(ALICE, ENDOWED_BALANCE), (BOB, ENDOWED_BALANCE)],
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
