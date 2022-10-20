#![cfg(test)]
use assets_registry::ASSETS_REGISTRY_ID;
use frame_support::{traits::GenesisBuild, PalletId};
use sp_io::TestExternalities;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub mod para;
pub mod relay;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const RELAYER_A: AccountId32 = AccountId32::new([2u8; 32]);
pub const RELAYER_B: AccountId32 = AccountId32::new([3u8; 32]);
pub const RELAYER_C: AccountId32 = AccountId32::new([4u8; 32]);
pub const ENDOWED_BALANCE: u128 = 100_000_000;
pub const TEST_THRESHOLD: u32 = 2;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(2),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(3),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		XcmConfig = relay::XcmConfig,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
			(3, ParaC),
		],
	}
}

pub type ParaBalances = pallet_balances::Pallet<para::Runtime>;
pub type ParaAssets = pallet_assets::Pallet<para::Runtime>;
pub type ParaAssetsRegistry = assets_registry::Pallet<para::Runtime>;
pub type ParaChainBridge = crate::chainbridge::Pallet<para::Runtime>;
pub type ParaXTransfer = crate::xtransfer::Pallet<para::Runtime>;
pub type ParaResourceIdGenSalt =
	<para::Runtime as crate::chainbridge::Config>::ResourceIdGenerationSalt;

pub fn para_ext(para_id: u32) -> TestExternalities {
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	let parachain_info_config = pallet_parachain_info::GenesisConfig {
		parachain_id: para_id.into(),
	};
	<pallet_parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	let bridge_account: AccountId32 = PalletId(*b"phala/bg").into_account_truncating();
	let assets_registry_account: AccountId32 = ASSETS_REGISTRY_ID.into_account_truncating();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(bridge_account, ENDOWED_BALANCE),
			(assets_registry_account, ENDOWED_BALANCE),
			(RELAYER_A, ENDOWED_BALANCE),
			(ALICE, ENDOWED_BALANCE),
			(BOB, ENDOWED_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, 1_000), (BOB, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[allow(dead_code)]
pub fn take_events() -> Vec<para::RuntimeEvent> {
	use para::Runtime;

	let evt = frame_system::Pallet::<Runtime>::events()
		.into_iter()
		.map(|evt| evt.event)
		.collect::<Vec<_>>();
	println!("event(): {:?}", evt);
	frame_system::Pallet::<Runtime>::reset_events();
	evt
}

pub fn para_last_event() -> para::RuntimeEvent {
	use para::Runtime;

	frame_system::Pallet::<Runtime>::events()
		.pop()
		.map(|e| e.event)
		.expect("Event expected")
}

pub fn para_expect_event<E: Into<para::RuntimeEvent>>(e: E) {
	assert_eq!(para_last_event(), e.into());
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn para_assert_events(mut expected: Vec<para::RuntimeEvent>) {
	use para::Runtime;

	let mut actual: Vec<para::RuntimeEvent> = frame_system::Pallet::<Runtime>::events()
		.iter()
		.map(|e| e.event.clone())
		.collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("event expected");
		assert_eq!(next, evt.into(), "Events don't match");
	}
}
