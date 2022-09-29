use crate::{
	attestation::{Attestation, AttestationValidator, Error as AttestationError, IasFields},
	basepool, mining, mq, pawnshop, registry, stakepoolv2, vault,
};

use frame_support::{
	ord_parameter_types,
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU64, EqualPrivilegeOnly, GenesisBuild, SortedMembers,
	},
};
use frame_support_test::TestRandomness;
use frame_system as system;
use phala_types::messaging::Message;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

use frame_system::EnsureRoot;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Uniques: pallet_uniques::{Pallet, Storage, Event<T>},
		RmrkCore: pallet_rmrk_core::{Pallet, Call, Event<T>},
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		// Pallets to test
		PhalaMq: mq::{Pallet, Call},
		PhalaRegistry: registry::{Pallet, Event<T>, Storage, Config<T>},
		PhalaMining: mining::{Pallet, Event<T>, Storage, Config},
		PhalaStakePool: stakepoolv2::{Pallet, Event<T>},
		PhalaVault: vault::{Pallet, Event<T>},
		PhalaPawnshop: pawnshop::{Pallet, Event<T>},
		PhalaBasePool: basepool::{Pallet, Event<T>},
	}
);

impl crate::PhalaConfig for Test {
	type Currency = Balances;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 2;
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 20;
	pub const MinimumPeriod: u64 = 1;
	pub const ExpectedBlockTimeSec: u32 = 12;
	pub const MinMiningStaking: Balance = 1 * DOLLARS;
	pub const MinContribution: Balance = 1 * CENTS;
	pub const MiningGracePeriod: u64 = 7 * 24 * 3600;
	pub const MinInitP: u32 = 1;
	pub const MiningEnabledByDefault: bool = true;
	pub const MaxPoolWorkers: u32 = 10;
	pub const VerifyPRuntime: bool = false;
	pub const VerifyRelaychainGenesisBlockHash: bool = true;
}
impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
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

impl pallet_scheduler::Config for Test {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = ();
	type ScheduleOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type MaxScheduledPerBlock = ();
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PreimageProvider = ();
	type NoPreimagePostponement = ();
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub const DOLLARS: Balance = 1_000_000_000_000;
pub const CENTS: Balance = DOLLARS / 100;
pub const DAYS: u64 = 24 * 3600;
pub const HOURS: u64 = 3600;

pub struct OneToFive;
impl SortedMembers<u64> for OneToFive {
	fn sorted_members() -> Vec<u64> {
		vec![1, 2, 3, 4, 5]
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u64) {}
}

impl mq::Config for Test {
	type QueueNotifyConfig = ();
	type CallMatcher = MqCallMatcher;
}

pub struct MqCallMatcher;
impl mq::CallMatcher<Test> for MqCallMatcher {
	fn match_call(call: &Call) -> Option<&mq::Call<Test>> {
		match call {
			Call::PhalaMq(mq_call) => Some(mq_call),
			_ => None,
		}
	}
}

impl registry::Config for Test {
	type Event = Event;
	type AttestationValidator = MockValidator;
	type UnixTime = Timestamp;
	type VerifyPRuntime = VerifyPRuntime;
	type VerifyRelaychainGenesisBlockHash = VerifyRelaychainGenesisBlockHash;
	type GovernanceOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

parameter_types! {
	pub const CollectionDeposit: Balance = 0; // 1 UNIT deposit to create collection
	pub const ItemDeposit: Balance = 0; // 1/100 UNIT deposit to create item
	pub const StringLimit: u32 = 52100;
	pub const KeyLimit: u32 = 32000; // Max 32 bytes per key
	pub const ValueLimit: u32 = 512000; // Max 64 bytes per value
	pub const UniquesMetadataDepositBase: Balance = 0;
	pub const AttributeDepositBase: Balance = 0;
	pub const DepositPerByte: Balance = 0;
}
impl pallet_uniques::Config for Test {
	type Event = Event;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type Locker = pallet_rmrk_core::Pallet<Test>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = UniquesMetadataDepositBase;
	type AttributeDepositBase = AttributeDepositBase;
	type DepositPerByte = DepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type WeightInfo = ();
}
parameter_types! {
	pub ClassBondAmount: Balance = 100;
	pub MaxMetadataLength: u32 = 256;
	pub const MaxRecursions: u32 = 10;
	pub const ResourceSymbolLimit: u32 = 10;
	pub const PartsLimit: u32 = 10;
	pub const MaxPriorities: u32 = 3;
	pub const CollectionSymbolLimit: u32 = 100;
	pub const MaxResourcesOnMint: u32 = 100;
}
impl pallet_rmrk_core::Config for Test {
	type Event = Event;
	type ProtocolOrigin = EnsureRoot<Self::AccountId>;
	type MaxRecursions = MaxRecursions;
	type ResourceSymbolLimit = ResourceSymbolLimit;
	type PartsLimit = PartsLimit;
	type MaxPriorities = MaxPriorities;
	type CollectionSymbolLimit = CollectionSymbolLimit;
	type MaxResourcesOnMint = MaxResourcesOnMint;
}

impl mining::Config for Test {
	type Event = Event;
	type ExpectedBlockTimeSec = ExpectedBlockTimeSec;
	type MinInitP = MinInitP;
	type Randomness = TestRandomness<Self>;
	type OnReward = PhalaStakePool;
	type OnUnbound = PhalaStakePool;
	type OnStopped = PhalaStakePool;
	type OnTreasurySettled = ();
	type UpdateTokenomicOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

parameter_types! {
	pub const PPhaAssetId: u32 = 1;
}

impl pawnshop::Config for Test {
	type Event = Event;
	type PPhaAssetId = PPhaAssetId;
	type PawnShopAccountId = ConstU64<1234>;
	type OnSlashed = ();
}

parameter_types! {
	pub const LaunchPeriod: u64 = 7 * DAYS;
	pub const VotingPeriod: u64 = 7 * DAYS;
	pub const FastTrackVotingPeriod: u64 = 3 * HOURS;
	pub const InstantAllowed: bool = true;
	pub const MinimumDeposit: Balance = 10 * DOLLARS;
	pub const EnactmentPeriod: u64 = 8 * DAYS;
	pub const CooloffPeriod: u64 = 7 * DAYS;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
	pub const PreimageByteDeposit: Balance = 1;
}

ord_parameter_types! {
	pub const One: u64 = 1;
	pub const Two: u64 = 2;
	pub const Three: u64 = 3;
	pub const Four: u64 = 4;
	pub const Five: u64 = 5;
	pub const Six: u64 = 6;
}

impl pallet_democracy::Config for Test {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = frame_system::EnsureRoot<Self::AccountId>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = frame_system::EnsureRoot<Self::AccountId>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = frame_system::EnsureRoot<Self::AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type InstantOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = frame_system::EnsureRoot<Self::AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BlacklistOrigin = frame_system::EnsureRoot<Self::AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cooloff period.
	type VetoOrigin = frame_system::EnsureSignedBy<OneToFive, u64>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type OperationalPreimageOrigin = frame_system::EnsureSignedBy<Six, u64>;
	type Slash = ();
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = ();
	type MaxProposals = MaxProposals;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1; // 1 Unit deposit to create asset
	pub const ApprovalDeposit: Balance = 1;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type AssetId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
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

impl stakepoolv2::Config for Test {
	type Event = Event;
	type MinContribution = MinContribution;
	type GracePeriod = MiningGracePeriod;
	type MiningEnabledByDefault = MiningEnabledByDefault;
	type MaxPoolWorkers = MaxPoolWorkers;
	type MiningSwitchOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BackfillOrigin = frame_system::EnsureRoot<Self::AccountId>;
}

impl vault::Config for Test {
	type Event = Event;
}

impl basepool::Config for Test {
	type Event = Event;
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

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	// Inject genesis storage
	let zero_pubkey = sp_core::sr25519::Public::from_raw([0u8; 32]);
	let zero_ecdh_pubkey = Vec::from(&[0u8; 32][..]);
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(1, 1000 * DOLLARS),
			(2, 2000 * DOLLARS),
			(3, 1000 * DOLLARS),
			(99, 1_000_000 * DOLLARS),
			(PhalaMining::account_id(), 690_000_000 * DOLLARS),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	crate::registry::GenesisConfig::<Test> {
		workers: vec![(zero_pubkey.clone(), zero_ecdh_pubkey, None)],
		gatekeepers: vec![(zero_pubkey.clone())],
		benchmark_duration: 0u32,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	GenesisBuild::<Test>::assimilate_storage(&crate::mining::GenesisConfig::default(), &mut t)
		.unwrap();
	sp_io::TestExternalities::new(t)
}

pub fn set_block_1() {
	System::set_block_number(1);
}

pub fn take_events() -> Vec<Event> {
	let evt = System::events()
		.into_iter()
		.map(|evt| evt.event)
		.collect::<Vec<_>>();
	println!("event(): {:?}", evt);
	System::reset_events();
	evt
}

pub fn take_messages() -> Vec<Message> {
	let messages = PhalaMq::messages();
	println!("messages(): {:?}", messages);
	mq::OutboundMessages::<Test>::kill();
	messages
}

use phala_types::{EcdhPublicKey, WorkerPublicKey};

pub fn worker_pubkey(i: u8) -> WorkerPublicKey {
	let mut raw = [0u8; 32];
	raw[31] = i;
	raw[30] = 1; // distinguish with the genesis config
	WorkerPublicKey::from_raw(raw)
}
pub fn ecdh_pubkey(i: u8) -> EcdhPublicKey {
	let mut raw = [0u8; 32];
	raw[31] = i;
	raw[30] = 1; // distinguish with the genesis config
	EcdhPublicKey(raw)
}

pub fn setup_relaychain_genesis_allowlist() {
	use frame_support::assert_ok;
	let sample: H256 = H256::repeat_byte(1);
	assert_ok!(PhalaRegistry::add_relaychain_genesis_block_hash(
		Origin::root(),
		sample
	));
}

/// Sets up `n` workers starting from 1, registered and benchmarked. All owned by account1.
pub fn setup_workers(n: u8) {
	use frame_support::assert_ok;
	for i in 1..=n {
		let worker = worker_pubkey(i);
		assert_ok!(PhalaRegistry::force_register_worker(
			Origin::root(),
			worker.clone(),
			ecdh_pubkey(1),
			Some(1)
		));
		PhalaRegistry::internal_set_benchmark(&worker, Some(1));
	}
}

/// Sets up `n` workers starting from 1, registered and benchmarked, owned by the corresponding
/// accounts.
pub fn setup_workers_linked_operators(n: u8) {
	use frame_support::assert_ok;
	for i in 1..=n {
		let worker = worker_pubkey(i);
		assert_ok!(PhalaRegistry::force_register_worker(
			Origin::root(),
			worker.clone(),
			ecdh_pubkey(1),
			Some(i as _)
		));
		PhalaRegistry::internal_set_benchmark(&worker, Some(1));
	}
}

pub fn elapse_seconds(sec: u64) {
	let now = Timestamp::get();
	Timestamp::set_timestamp(now + sec * 1000);
}

pub fn elapse_cool_down() {
	let now = Timestamp::get();
	Timestamp::set_timestamp(now + PhalaMining::cool_down_period() * 1000);
}