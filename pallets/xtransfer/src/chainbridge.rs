pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, IntoResourceId,
		CB_ASSET_KEY,
	};
	use codec::{Decode, Encode, EncodeLike};
	pub use frame_support::{
		pallet_prelude::*,
		traits::{tokens::fungibles::Inspect, Currency, StorageVersion},
		weights::GetDispatchInfo,
		PalletId, Parameter,
	};
	use frame_system::{self as system, pallet_prelude::*};
	use scale_info::TypeInfo;
	pub use sp_core::U256;
	use sp_runtime::traits::{AccountIdConversion, Dispatchable};
	use sp_runtime::RuntimeDebug;
	use sp_std::{
		convert::{From, Into, TryInto},
		prelude::*,
	};
	use xcm::latest::{
		prelude::*, AssetId as XcmAssetId, Fungibility::Fungible, MultiAsset, MultiLocation,
	};
	use xcm_executor::traits::TransactAsset;

	const LOG_TARGET: &str = "runtime::chainbridge";
	const DEFAULT_RELAYER_THRESHOLD: u32 = 1;
	const MODULE_ID: PalletId = PalletId(*b"phala/bg");

	pub type BridgeChainId = u8;
	pub type DepositNonce = u64;
	pub type ResourceId = [u8; 32];

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Helper function to concatenate a chain ID and some bytes to produce a resource ID.
	/// The common format is (31 bytes unique ID + 1 byte chain ID).
	pub fn derive_resource_id(chain: u8, id: &[u8]) -> ResourceId {
		let mut r_id: ResourceId = [0; 32];
		r_id[31] = chain; // last byte is chain id
		let range = if id.len() > 31 { 31 } else { id.len() }; // Use at most 31 bytes
		for i in 0..range {
			r_id[30 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
		}
		r_id
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub enum ProposalStatus {
		Initiated,
		Approved,
		Rejected,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub struct ProposalVotes<AccountId, BlockNumber> {
		pub votes_for: Vec<AccountId>,
		pub votes_against: Vec<AccountId>,
		pub status: ProposalStatus,
		pub expiry: BlockNumber,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub enum BridgeEvent {
		FungibleTransfer(BridgeChainId, DepositNonce, ResourceId, U256, Vec<u8>),
		NonFungibleTransfer(
			BridgeChainId,
			DepositNonce,
			ResourceId,
			Vec<u8>,
			Vec<u8>,
			Vec<u8>,
		),
		GenericTransfer(BridgeChainId, DepositNonce, ResourceId, Vec<u8>),
	}

	impl<A: PartialEq, B: PartialOrd + Default> ProposalVotes<A, B> {
		/// Attempts to mark the proposal as approve or rejected.
		/// Returns true if the status changes from active.
		pub fn try_to_complete(&mut self, threshold: u32, total: u32) -> ProposalStatus {
			if self.votes_for.len() >= threshold as usize {
				self.status = ProposalStatus::Approved;
				ProposalStatus::Approved
			} else if total >= threshold && self.votes_against.len() as u32 + threshold > total {
				self.status = ProposalStatus::Rejected;
				ProposalStatus::Rejected
			} else {
				ProposalStatus::Initiated
			}
		}

		/// Returns true if the proposal has been rejected or approved, otherwise false.
		fn is_complete(&self) -> bool {
			self.status != ProposalStatus::Initiated
		}

		/// Returns true if `who` has voted for or against the proposal
		fn has_voted(&self, who: &A) -> bool {
			self.votes_for.contains(who) || self.votes_against.contains(who)
		}

		/// Return true if the expiry time has been reached
		fn is_expired(&self, now: B) -> bool {
			self.expiry <= now
		}
	}

	impl<AccountId, BlockNumber: Default> Default for ProposalVotes<AccountId, BlockNumber> {
		fn default() -> Self {
			Self {
				votes_for: vec![],
				votes_against: vec![],
				status: ProposalStatus::Initiated,
				expiry: BlockNumber::default(),
			}
		}
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::Origin>;
		/// Proposed dispatchable call
		type Proposal: Parameter
			+ Dispatchable<Origin = Self::Origin>
			+ EncodeLike
			+ GetDispatchInfo;
		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a set of bridged chains.
		#[pallet::constant]
		type BridgeChainId: Get<BridgeChainId>;

		/// Currency impl
		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type ProposalLifetime: Get<Self::BlockNumber>;

		/// Check whether an asset is PHA
		type NativeAssetChecker: NativeAssetChecker;

		/// Execution price in PHA
		type NativeExecutionPrice: Get<u128>;

		/// Execution price information
		type ExecutionPriceInfo: Get<Vec<(XcmAssetId, u128)>>;

		/// Treasury account to receive assets fee
		type TreasuryAccount: Get<Self::AccountId>;

		/// Asset adapter to do withdraw, deposit etc.
		type FungibleAdapter: TransactAsset;

		/// Fungible assets registry
		type AssetsRegistry: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Vote threshold has changed (new_threshold)
		RelayerThresholdChanged(u32),
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted(BridgeChainId),
		/// Relayer added to set
		RelayerAdded(T::AccountId),
		/// Relayer removed from set
		RelayerRemoved(T::AccountId),
		/// FungibleTransfer is for relaying fungibles (dest_id, nonce, resource_id, amount, recipient)
		FungibleTransfer(BridgeChainId, DepositNonce, ResourceId, U256, Vec<u8>),
		/// NonFungibleTransfer is for relaying NFTs (dest_id, nonce, resource_id, token_id, recipient, metadata)
		NonFungibleTransfer(
			BridgeChainId,
			DepositNonce,
			ResourceId,
			Vec<u8>,
			Vec<u8>,
			Vec<u8>,
		),
		/// GenericTransfer is for a generic data payload (dest_id, nonce, resource_id, metadata)
		GenericTransfer(BridgeChainId, DepositNonce, ResourceId, Vec<u8>),
		/// Vote submitted in favour of proposal
		VoteFor(BridgeChainId, DepositNonce, T::AccountId),
		/// Vot submitted against proposal
		VoteAgainst(BridgeChainId, DepositNonce, T::AccountId),
		/// Voting successful for a proposal
		ProposalApproved(BridgeChainId, DepositNonce),
		/// Voting rejected a proposal
		ProposalRejected(BridgeChainId, DepositNonce),
		/// Execution of call succeeded
		ProposalSucceeded(BridgeChainId, DepositNonce),
		/// Execution of call failed
		ProposalFailed(BridgeChainId, DepositNonce),
		FeeUpdated {
			dest_id: BridgeChainId,
			min_fee: u128,
			fee_scale: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Provided chain Id is not valid
		InvalidChainId,
		/// Relayer threshold cannot be 0
		InvalidThreshold,
		/// Interactions with this chain is not permitted
		ChainNotWhitelisted,
		/// Chain has already been enabled
		ChainAlreadyWhitelisted,
		/// Relayer already in set
		RelayerAlreadyExists,
		/// Provided accountId is not a relayer
		RelayerInvalid,
		/// Protected operation, must be performed by relayer
		MustBeRelayer,
		/// Relayer has already submitted some vote for this proposal
		RelayerAlreadyVoted,
		/// A proposal with these parameters has already been submitted
		ProposalAlreadyExists,
		/// No proposal with the ID was found
		ProposalDoesNotExist,
		/// Cannot complete proposal, needs more votes
		ProposalNotComplete,
		/// Proposal has either failed or succeeded
		ProposalAlreadyComplete,
		/// Lifetime of proposal has been exceeded
		ProposalExpired,
		/// Got wrong paremeter when update fee
		InvalidFeeOption,
		/// Unkonwn asset
		ExtractAssetFailed,
		/// Unknown destnation
		ExtractDestFailed,
		/// Asset can not pay as fee
		CannotPayAsFee,
		/// Transfer failed
		TransactFailed,
		/// Infusficient balance to withdraw
		InsufficientBalance,
		/// Too expensive fee for withdrawn asset 
		FeeTooExpensive,
		/// Can not extract asset reserve location
		CannotDetermineReservedLocation,
		/// Can not extract dest location
		DestUnrecognized,
		/// Assets not registered through pallet-assets or pallet-uniques
		AssetNotRegistered,
		/// Convertion failed from resource id
		AssetConversionFailed,
		/// Function unimplemented
		Unimplemented,
		/// Can not transfer assets to dest due to some reasons
		CannotDepositAsset,
	}

	#[pallet::storage]
	#[pallet::getter(fn chains)]
	pub type ChainNonces<T> = StorageMap<_, Blake2_256, BridgeChainId, DepositNonce>;

	#[pallet::type_value]
	pub fn DefaultRelayerThresholdValue() -> u32 {
		DEFAULT_RELAYER_THRESHOLD
	}

	#[pallet::storage]
	#[pallet::getter(fn relayer_threshold)]
	pub type RelayerThreshold<T> = StorageValue<_, u32, ValueQuery, DefaultRelayerThresholdValue>;

	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageMap<_, Blake2_256, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayer_count)]
	pub type RelayerCount<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub type Votes<T: Config> = StorageDoubleMap<
		_,
		Blake2_256,
		BridgeChainId,
		Blake2_256,
		(DepositNonce, T::Proposal),
		ProposalVotes<T::AccountId, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_events)]
	pub type BridgeEvents<T> = StorageValue<_, Vec<BridgeEvent>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> = StorageMap<_, Twox64Concat, BridgeChainId, (u128, u32)>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			// Clear all bridge transfer data
			BridgeEvents::<T>::kill();
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		/// Sets the vote threshold for proposals.
		///
		/// This threshold is used to determine how many votes are required
		/// before a proposal is executed.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn set_threshold(origin: OriginFor<T>, threshold: u32) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			Self::set_relayer_threshold(threshold)
		}

		/// Enables a chain ID as a source or destination for a bridge transfer.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn whitelist_chain(origin: OriginFor<T>, id: BridgeChainId) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			Self::whitelist(id)
		}

		/// Adds a new relayer to the relayer set.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn add_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			Self::register_relayer(v)
		}

		/// Removes an existing relayer from the set.
		///
		/// # <weight>
		/// - O(1) lookup and removal
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn remove_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			Self::unregister_relayer(v)
		}

		/// Change extra bridge transfer fee that user should pay
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn update_fee(
			origin: OriginFor<T>,
			min_fee: u128,
			fee_scale: u32,
			dest_id: BridgeChainId,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated {
				dest_id,
				min_fee,
				fee_scale,
			});
			Ok(())
		}

		/// Commits a vote in favour of the provided proposal.
		///
		/// If a proposal with the given nonce and source chain ID does not already exist, it will
		/// be created with an initial vote in favour from the caller.
		///
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(dispatch_info.weight + 195_000_000, dispatch_info.class, Pays::Yes)
		})]
		pub fn acknowledge_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			_r_id: ResourceId,
			call: Box<<T as Config>::Proposal>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
			ensure!(
				Self::chain_whitelisted(src_id),
				Error::<T>::ChainNotWhitelisted
			);

			Self::vote_for(who, nonce, src_id, call)
		}

		/// Commits a vote against a provided proposal.
		///
		/// # <weight>
		/// - Fixed, since execution of proposal should not be included
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn reject_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			_r_id: ResourceId,
			call: Box<<T as Config>::Proposal>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
			ensure!(
				Self::chain_whitelisted(src_id),
				Error::<T>::ChainNotWhitelisted
			);

			Self::vote_against(who, nonce, src_id, call)
		}

		/// Evaluate the state of a proposal given the current vote threshold.
		///
		/// A proposal with enough votes will be either executed or cancelled, and the status
		/// will be updated accordingly.
		///
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight({
			let dispatch_info = prop.get_dispatch_info();
			(dispatch_info.weight + 195_000_000, dispatch_info.class, Pays::Yes)
		})]
		pub fn eval_vote_state(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			prop: Box<<T as Config>::Proposal>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			Self::try_resolve_proposal(nonce, src_id, prop)
		}

		/// Triggered by a initial transfer on source chain, executed by relayer when proposal was resolved.
		#[pallet::weight(195_000_000)]
		pub fn handle_fungible_transfer(
			origin: OriginFor<T>,
			dest: Vec<u8>,
			amount: BalanceOf<T>,
			rid: ResourceId,
		) -> DispatchResult {
			EnsureBridge::<T>::ensure_origin(origin.clone())?;

			// For solo chain assets, we encode solo chain id as the first byte of resourceId
			let src_chainid: BridgeChainId = Self::get_chainid(&rid);
			let src_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(src_chainid.into()),
				),
			)
				.into();
			let dest_location: MultiLocation =
				Decode::decode(&mut dest.as_slice()).map_err(|_| Error::<T>::DestUnrecognized)?;
			let asset_location = Self::rid_to_location(&rid)?;
			let asset_reserve_location = asset_location
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;

			log::trace!(
				target: LOG_TARGET,
				"Reserve location of assset ${:?}, reserve location of source: {:?}.",
				&asset_reserve_location,
				&src_reserve_location,
			);

			// We received asset send from non-reserve chain, which reserved
			// in the local or other parachains/relaychain. That means we had
			// reserved the asset in a reserve account while it was transfered
			// the the source chain, so here we need withdraw/burn from the reserve
			// account in advance.
			//
			// Note: If we received asset send from its reserve chain, we just need
			// mint the same amount of asset at local
			if asset_reserve_location != src_reserve_location {
				let reserve_account: T::AccountId = if rid == Self::gen_pha_rid(src_chainid) {
					// PHA need to be released from bridge account due to historical reason
					MODULE_ID.into_account()
				} else {
					src_reserve_location.into_account().into()
				};
				T::FungibleAdapter::withdraw_asset(
					&(asset_location.clone(), amount.into()).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: reserve_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
				log::trace!(
                    target: LOG_TARGET,
                    "Reserve of asset and src dismatch, withdraw asset form source reserve location.",
                );
			}

			// If dest_location is not a local account, adapter will forward it to other bridges
			T::FungibleAdapter::deposit_asset(
				&(asset_location.clone(), amount.into()).into(),
				&dest_location,
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		// *** Utility methods ***

		/// Checks if who is a relayer
		pub fn is_relayer(who: &T::AccountId) -> bool {
			Self::relayers(who)
		}

		/// Provides an AccountId for the pallet.
		/// This is used both as an origin check and deposit/withdrawal account.
		pub fn account_id() -> T::AccountId {
			MODULE_ID.into_account()
		}

		/// Checks if a chain exists as a whitelisted destination
		pub fn chain_whitelisted(id: BridgeChainId) -> bool {
			Self::chains(id).is_some()
		}

		/// Increments the deposit nonce for the specified chain ID
		fn bump_nonce(id: BridgeChainId) -> DepositNonce {
			let nonce = Self::chains(id).unwrap_or_default() + 1;
			ChainNonces::<T>::insert(id, nonce);
			nonce
		}

		// *** Admin methods ***

		/// Set a new voting threshold
		pub fn set_relayer_threshold(threshold: u32) -> DispatchResult {
			ensure!(threshold > 0, Error::<T>::InvalidThreshold);
			RelayerThreshold::<T>::put(threshold);
			Self::deposit_event(Event::RelayerThresholdChanged(threshold));
			Ok(())
		}

		/// Whitelist a chain ID for transfer
		pub fn whitelist(id: BridgeChainId) -> DispatchResult {
			// Cannot whitelist this chain
			ensure!(id != T::BridgeChainId::get(), Error::<T>::InvalidChainId);
			// Cannot whitelist with an existing entry
			ensure!(
				!Self::chain_whitelisted(id),
				Error::<T>::ChainAlreadyWhitelisted
			);
			ChainNonces::<T>::insert(&id, 0);
			Self::deposit_event(Event::ChainWhitelisted(id));
			Ok(())
		}

		/// Adds a new relayer to the set
		pub fn register_relayer(relayer: T::AccountId) -> DispatchResult {
			ensure!(
				!Self::is_relayer(&relayer),
				Error::<T>::RelayerAlreadyExists
			);
			Relayers::<T>::insert(&relayer, true);
			RelayerCount::<T>::mutate(|i| *i += 1);

			Self::deposit_event(Event::RelayerAdded(relayer));
			Ok(())
		}

		/// Removes a relayer from the set
		pub fn unregister_relayer(relayer: T::AccountId) -> DispatchResult {
			ensure!(Self::is_relayer(&relayer), Error::<T>::RelayerInvalid);
			Relayers::<T>::remove(&relayer);
			RelayerCount::<T>::mutate(|i| *i -= 1);
			Self::deposit_event(Event::RelayerRemoved(relayer));
			Ok(())
		}

		// *** Proposal voting and execution methods ***

		/// Commits a vote for a proposal. If the proposal doesn't exist it will be created.
		fn commit_vote(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			prop: Box<T::Proposal>,
			in_favour: bool,
		) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			let mut votes = match Votes::<T>::get(src_id, (nonce, prop.clone())) {
				Some(v) => v,
				None => ProposalVotes {
					expiry: now + T::ProposalLifetime::get(),
					..Default::default()
				},
			};

			// Ensure the proposal isn't complete and relayer hasn't already voted
			ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
			ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);
			ensure!(!votes.has_voted(&who), Error::<T>::RelayerAlreadyVoted);

			if in_favour {
				votes.votes_for.push(who.clone());
				Self::deposit_event(Event::VoteFor(src_id, nonce, who));
			} else {
				votes.votes_against.push(who.clone());
				Self::deposit_event(Event::VoteAgainst(src_id, nonce, who));
			}

			Votes::<T>::insert(src_id, (nonce, prop), votes);

			Ok(())
		}

		/// Attempts to finalize or cancel the proposal if the vote count allows.
		fn try_resolve_proposal(
			nonce: DepositNonce,
			src_id: BridgeChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResult {
			if let Some(mut votes) = Votes::<T>::get(src_id, (nonce, prop.clone())) {
				let now = <frame_system::Pallet<T>>::block_number();
				ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
				ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);

				let status =
					votes.try_to_complete(RelayerThreshold::<T>::get(), RelayerCount::<T>::get());
				Votes::<T>::insert(src_id, (nonce, prop.clone()), votes);

				match status {
					ProposalStatus::Approved => Self::finalize_execution(src_id, nonce, prop),
					ProposalStatus::Rejected => Self::cancel_execution(src_id, nonce),
					_ => Ok(()),
				}
			} else {
				Err(Error::<T>::ProposalDoesNotExist.into())
			}
		}

		/// Commits a vote in favour of the proposal and executes it if the vote threshold is met.
		fn vote_for(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResult {
			Self::commit_vote(who, nonce, src_id, prop.clone(), true)?;
			Self::try_resolve_proposal(nonce, src_id, prop)
		}

		/// Commits a vote against the proposal and cancels it if more than (relayers.len() - threshold)
		/// votes against exist.
		fn vote_against(
			who: T::AccountId,
			nonce: DepositNonce,
			src_id: BridgeChainId,
			prop: Box<T::Proposal>,
		) -> DispatchResult {
			Self::commit_vote(who, nonce, src_id, prop.clone(), false)?;
			Self::try_resolve_proposal(nonce, src_id, prop)
		}

		/// Execute the proposal and signals the result as an event
		#[allow(clippy::boxed_local)]
		fn finalize_execution(
			src_id: BridgeChainId,
			nonce: DepositNonce,
			call: Box<T::Proposal>,
		) -> DispatchResult {
			Self::deposit_event(Event::ProposalApproved(src_id, nonce));
			call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;
			Self::deposit_event(Event::ProposalSucceeded(src_id, nonce));
			Ok(())
		}

		/// Cancels a proposal.
		fn cancel_execution(src_id: BridgeChainId, nonce: DepositNonce) -> DispatchResult {
			Self::deposit_event(Event::ProposalRejected(src_id, nonce));
			Ok(())
		}

		fn extract_fungible(asset: MultiAsset) -> Option<(MultiLocation, u128)> {
			return match (asset.fun, asset.id) {
				(Fungible(amount), Concrete(location)) => Some((location, amount)),
				_ => None,
			};
		}

		fn extract_dest(dest: &MultiLocation) -> Option<(BridgeChainId, Vec<u8>)> {
			return match (dest.parents, &dest.interior) {
				// Destnation is a foreign chain. Forward it through the bridge
				(
					0,
					Junctions::X3(GeneralKey(_), GeneralIndex(chain_id), GeneralKey(recipient)),
				) => {
					if let Some(chain_id) = TryInto::<BridgeChainId>::try_into(*chain_id).ok() {
						// We don't verify cb_key here because can_deposit_asset did it.
						Some((chain_id, recipient.to_vec()))
					} else {
						None
					}
				}
				_ => None,
			};
		}

		pub fn estimate_fee_in_pha(dest_id: BridgeChainId, amount: u128) -> Option<u128> {
			return match Self::bridge_fee(dest_id) {
				Some((min_fee, fee_scale)) => {
					let fee_estimated = amount
						.saturating_mul(fee_scale as u128)
						.saturating_div(1000u128);
					if fee_estimated > min_fee {
						Some(fee_estimated)
					} else {
						Some(min_fee)
					}
				}
				_ => None,
			};
		}

		pub fn to_e12(amount: u128, decimals: u8) -> u128 {
			if decimals > 12 {
				amount.saturating_div(10u128.saturating_pow(decimals as u32 - 12))
			} else {
				amount.saturating_mul(10u128.saturating_pow(12 - decimals as u32))
			}
		}

		pub fn from_e12(amount: u128, decimals: u8) -> u128 {
			if decimals > 12 {
				amount.saturating_mul(10u128.saturating_pow(decimals as u32 - 12))
			} else {
				amount.saturating_div(10u128.saturating_pow(12 - decimals as u32))
			}
		}

		pub fn convert_fee_from_pha(fee_in_pha: u128, price: u128, decimals: u8) -> u128 {
			let fee_e12: u128 = fee_in_pha * price / T::NativeExecutionPrice::get();
			Self::from_e12(fee_e12, decimals)
		}

		pub fn rid_to_location(rid: &[u8; 32]) -> Result<MultiLocation, DispatchError> {
			let src_chainid: BridgeChainId = Self::get_chainid(rid);
			if *rid == Self::gen_pha_rid(src_chainid) {
				Ok(MultiLocation::here())
			} else {
				let location = T::AssetsRegistry::lookup_by_resource_id(&rid)
					.ok_or(Error::<T>::AssetConversionFailed)?;
				Ok(location)
			}
		}

		pub fn rid_to_assetid(
			rid: &[u8; 32],
		) -> Result<<T as pallet_assets::Config>::AssetId, DispatchError> {
			let src_chainid: BridgeChainId = Self::get_chainid(rid);
			// PHA based on pallet_balances, not pallet_assets
			if *rid == Self::gen_pha_rid(src_chainid) {
				return Err(Error::<T>::AssetNotRegistered.into());
			}
			let asset_location: MultiLocation = T::AssetsRegistry::lookup_by_resource_id(&rid)
				.ok_or(Error::<T>::AssetConversionFailed)?;
			let asset_id: <T as pallet_assets::Config>::AssetId =
				T::AssetsRegistry::id(&asset_location).ok_or(Error::<T>::AssetNotRegistered)?;
			Ok(asset_id)
		}

		pub fn gen_pha_rid(chain_id: BridgeChainId) -> ResourceId {
			MultiLocation::here().into_rid(chain_id)
		}

		pub fn get_chainid(rid: &ResourceId) -> BridgeChainId {
			rid[0]
		}

		fn get_fee(chain_id: BridgeChainId, asset: &MultiAsset) -> Option<u128> {
			if let Some((location, amount)) = Self::extract_fungible(asset.clone()) {
				let decimals = if T::NativeAssetChecker::is_native_asset(asset) {
					12
				} else {
					let id = T::AssetsRegistry::id(&location.clone())?;
					T::AssetsRegistry::decimals(&id).unwrap_or(12)
				};
				if let Some(fee_in_pha) =
					Self::estimate_fee_in_pha(chain_id, (Self::to_e12(amount, decimals)).into())
				{
					if T::NativeAssetChecker::is_native_asset(asset) {
						Some(fee_in_pha)
					} else {
						let fee_prices = T::ExecutionPriceInfo::get();
						let fee_in_asset = fee_prices
							.iter()
							.position(|(fee_asset_id, _)| {
								fee_asset_id == &Concrete(location.clone())
							})
							.map(|idx| {
								Self::convert_fee_from_pha(fee_in_pha, fee_prices[idx].1, decimals)
							});
						fee_in_asset
					}
				} else {
					None
				}
			} else {
				None
			}
			// TODO: Calculate NonFungible asset fee
		}

		fn check_balance(sender: T::AccountId, asset: &MultiAsset, amount: u128) -> DispatchResult {
			let balance: u128 = if T::NativeAssetChecker::is_native_asset(asset) {
				<T as Config>::Currency::free_balance(&sender).into()
			} else {
				let (asset_location, _) =
					Self::extract_fungible(asset.clone()).ok_or(Error::<T>::ExtractAssetFailed)?;
				let asset_id: <T as pallet_assets::Config>::AssetId =
					T::AssetsRegistry::id(&asset_location).ok_or(Error::<T>::AssetNotRegistered)?;
				let reducible_balance = <pallet_assets::pallet::Pallet<T>>::reducible_balance(
					asset_id.into(),
					&sender,
					false,
				);
				reducible_balance.into()
			};

			if balance >= amount {
				Ok(())
			} else {
				Err(Error::<T>::InsufficientBalance.into())
			}
		}
	}

	impl<T: Config> BridgeChecker for Pallet<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool {
			return match (
				Self::extract_fungible(asset.clone()),
				Self::extract_dest(&dest),
			) {
				(Some((asset_location, _)), Some((dest_id, _))) => {
					let rid = asset_location.clone().into_rid(dest_id);
					// Verify if dest chain has been whitelisted
					if Self::chain_whitelisted(dest_id) == false {
						return false;
					}

					// Verify if destination has fee set
					if BridgeFee::<T>::contains_key(&dest_id) == false {
						return false;
					}

					// Verify if asset was registered if is not native.
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& T::AssetsRegistry::id(&asset_location) == None
					{
						return false;
					}

					// Verify if asset was enabled chainbridge transfer if is not native
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& Self::rid_to_assetid(&rid).is_err()
					{
						return false;
					}

					true
				}
				_ => false,
			};
			// TODO: NonFungible verification
		}

		fn can_send_data(_data: &Vec<u8>, _dest: MultiLocation) -> bool {
			// TODO: impl
			return false;
		}
	}

	pub struct BridgeTransactImpl<T>(PhantomData<T>);
	impl<T: Config> BridgeTransact for BridgeTransactImpl<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn new() -> Self {
			Self(PhantomData)
		}

		/// Initiates a transfer of a fungible asset out of the chain. This should be called by another pallet.
		fn transfer_fungible(
			&self,
			sender: [u8; 32],
			asset: MultiAsset,
			dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			// Check if we can deposit asset into dest.
			ensure!(
				Pallet::<T>::can_deposit_asset(asset.clone(), dest.clone()),
				Error::<T>::CannotDepositAsset
			);

			let (asset_location, amount) = Pallet::<T>::extract_fungible(asset.clone())
				.ok_or(Error::<T>::ExtractAssetFailed)?;
			let (dest_id, recipient) =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::ExtractDestFailed)?;
			let resource_id = asset_location.clone().into_rid(dest_id);

			log::trace!(
				target: LOG_TARGET,
				" Chainbridge fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}.",
				sender,
				&asset,
				&dest,
			);

			let fee = Pallet::<T>::get_fee(
				dest_id,
				&(
					Concrete(asset_location.clone().into()),
					Fungible(amount.into()),
				)
					.into(),
			)
			.ok_or(Error::<T>::CannotPayAsFee)?;
			// No need to transfer to to dest chains if it's not enough to pay fee.
			ensure!(amount > fee, Error::<T>::FeeTooExpensive);
			// Ensure we have sufficient free balance
			Pallet::<T>::check_balance(sender.into(), &asset, amount)?;

			// Withdraw `amount` of asset from sender
			T::FungibleAdapter::withdraw_asset(
				&(asset.id.clone(), Fungible(amount.into())).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: sender,
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `fee` of asset to treasury account
			T::FungibleAdapter::deposit_asset(
				&(asset.id.clone(), Fungible(fee.into())).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: T::TreasuryAccount::get().into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `amount - fee` of asset to reserve account if asset is not reserved in dest.
			let dest_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(dest_id.into()),
				),
			)
				.into();
			let asset_reserve_location = asset_location
				.clone()
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;
			let reserve_account = if T::NativeAssetChecker::is_native_asset(&asset) {
				MODULE_ID.into_account()
			} else {
				dest_reserve_location.clone().into_account()
			};
			if T::NativeAssetChecker::is_native_asset(&asset)
				|| asset_reserve_location != dest_reserve_location
			{
				T::FungibleAdapter::deposit_asset(
					&(asset.id.clone(), Fungible((amount - fee).into())).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: reserve_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Notify relayer the crosschain transfer
			let nonce = Pallet::<T>::bump_nonce(dest_id);
			BridgeEvents::<T>::append(BridgeEvent::FungibleTransfer(
				dest_id,
				nonce,
				resource_id,
				U256::from(amount - fee),
				recipient.clone(),
			));
			Pallet::<T>::deposit_event(Event::FungibleTransfer(
				dest_id,
				nonce,
				resource_id,
				U256::from(amount - fee),
				recipient,
			));
			Ok(())
		}

		/// Initiates a transfer of a nonfungible asset out of the chain. This should be called by another pallet.
		fn transfer_nonfungible(
			&self,
			_sender: [u8; 32],
			_asset: MultiAsset,
			_dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}

		/// Initiates a transfer of generic data out of the chain. This should be called by another pallet.
		fn transfer_generic(
			&self,
			_sender: [u8; 32],
			_data: &Vec<u8>,
			_dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}
	}

	/// Simple ensure origin for the bridge account
	pub struct EnsureBridge<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> EnsureOrigin<T::Origin> for EnsureBridge<T> {
		type Success = T::AccountId;
		fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
			let bridge_account = MODULE_ID.into_account();
			o.into().and_then(|o| match o {
				system::RawOrigin::Signed(who) if who == bridge_account => Ok(bridge_account),
				r => Err(T::Origin::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn successful_origin() -> T::Origin {
			let bridge_account = MODULE_ID.into_account();
			T::Origin::from(system::RawOrigin::Signed(bridge_account))
		}
	}

	#[cfg(test)]
	mod test {
		use super::*;
		use crate::chainbridge::Error as ChainbridgeError;
		use crate::chainbridge::Event as ChainbridgeEvent;
		use crate::mock::para::*;
		use crate::mock::para::{Call, Event, Runtime};
		use crate::mock::{
			para_assert_events, para_expect_event, para_ext, ParaA, ParaChainBridge as ChainBridge,
			TestNet, ALICE, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C, TEST_THRESHOLD,
		};
		use assets_registry::*;
		use frame_support::{assert_noop, assert_ok};
		use xcm_simulator::TestExt;

		pub fn new_test_ext_initialized(
			src_id: BridgeChainId,
			_r_id: ResourceId,
			_resource: Vec<u8>,
		) -> sp_io::TestExternalities {
			let mut t = para_ext(1);
			t.execute_with(|| {
				// Set and check threshold
				assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD));
				assert_eq!(ChainBridge::relayer_threshold(), TEST_THRESHOLD);
				// Add relayers
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_C));
				// Whitelist chain
				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), src_id));
			});
			t
		}

		#[test]
		fn derive_ids() {
			let chain = 1;
			let id = [
				0x21, 0x60, 0x5f, 0x71, 0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0,
				0x24, 0xb7, 0xb1, 0x09, 0x99, 0xf4,
			];
			let r_id = derive_resource_id(chain, &id);
			let expected = [
				0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x21, 0x60, 0x5f, 0x71,
				0x84, 0x5f, 0x37, 0x2a, 0x9e, 0xd8, 0x42, 0x53, 0xd2, 0xd0, 0x24, 0xb7, 0xb1, 0x09,
				0x99, 0xf4, chain,
			];
			assert_eq!(r_id, expected);
		}

		#[test]
		fn complete_proposal_approved() {
			let mut prop = ProposalVotes {
				votes_for: vec![1, 2],
				votes_against: vec![3],
				status: ProposalStatus::Initiated,
				expiry: ProposalLifetime::get(),
			};

			prop.try_to_complete(2, 3);
			assert_eq!(prop.status, ProposalStatus::Approved);
		}

		#[test]
		fn complete_proposal_rejected() {
			let mut prop = ProposalVotes {
				votes_for: vec![1],
				votes_against: vec![2, 3],
				status: ProposalStatus::Initiated,
				expiry: ProposalLifetime::get(),
			};

			prop.try_to_complete(2, 3);
			assert_eq!(prop.status, ProposalStatus::Rejected);
		}

		#[test]
		fn complete_proposal_bad_threshold() {
			let mut prop = ProposalVotes {
				votes_for: vec![1, 2],
				votes_against: vec![],
				status: ProposalStatus::Initiated,
				expiry: ProposalLifetime::get(),
			};

			prop.try_to_complete(3, 2);
			assert_eq!(prop.status, ProposalStatus::Initiated);

			let mut prop = ProposalVotes {
				votes_for: vec![],
				votes_against: vec![1, 2],
				status: ProposalStatus::Initiated,
				expiry: ProposalLifetime::get(),
			};

			prop.try_to_complete(3, 2);
			assert_eq!(prop.status, ProposalStatus::Initiated);
		}

		#[test]
		fn whitelist_chain() {
			TestNet::reset();

			ParaA::execute_with(|| {
				assert!(!ChainBridge::chain_whitelisted(0));

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), 0));
				assert_noop!(
					ChainBridge::whitelist_chain(Origin::root(), TestChainId::get()),
					ChainbridgeError::<Runtime>::InvalidChainId
				);

				para_assert_events(vec![Event::ChainBridge(
					ChainbridgeEvent::ChainWhitelisted(0),
				)]);
			})
		}

		#[test]
		fn set_get_threshold() {
			TestNet::reset();

			ParaA::execute_with(|| {
				assert_eq!(<RelayerThreshold<Runtime>>::get(), 1);

				assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD));
				assert_eq!(<RelayerThreshold<Runtime>>::get(), TEST_THRESHOLD);

				assert_ok!(ChainBridge::set_threshold(Origin::root(), 5));
				assert_eq!(<RelayerThreshold<Runtime>>::get(), 5);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::RelayerThresholdChanged(TEST_THRESHOLD)),
					Event::ChainBridge(ChainbridgeEvent::RelayerThresholdChanged(5)),
				]);
			})
		}

		#[test]
		fn asset_transfer_invalid_chain() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let chain_id: BridgeChainId = 2;
				let bad_dest_id: BridgeChainId = 3;
				let resource_id = [4; 32];

				assert_ok!(ChainBridge::whitelist_chain(
					Origin::root(),
					chain_id.clone()
				));
				para_assert_events(vec![Event::ChainBridge(
					ChainbridgeEvent::ChainWhitelisted(chain_id.clone()),
				)]);

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_noop!(
					bridge_impl.transfer_fungible(
						ALICE.into(),
						(Concrete(MultiLocation::new(0, Here)), Fungible(100u128)).into(),
						MultiLocation::new(
							0,
							X3(
								GeneralKey(b"cb".to_vec()),
								GeneralIndex(bad_dest_id.into()),
								GeneralKey(b"recipient".to_vec())
							)
						),
						None,
					),
					// Can not pass can_deposit_asset check if chain not been whitelisted
					ChainbridgeError::<Runtime>::CannotDepositAsset
				);
			})
		}

		#[test]
		fn add_remove_relayer() {
			TestNet::reset();

			ParaA::execute_with(|| {
				assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
				assert_eq!(ChainBridge::relayer_count(), 0);

				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_C));
				assert_eq!(ChainBridge::relayer_count(), 3);

				// Already exists
				assert_noop!(
					ChainBridge::add_relayer(Origin::root(), RELAYER_A),
					ChainbridgeError::<Runtime>::RelayerAlreadyExists
				);

				// Confirm removal
				assert_ok!(ChainBridge::remove_relayer(Origin::root(), RELAYER_B));
				assert_eq!(ChainBridge::relayer_count(), 2);
				assert_noop!(
					ChainBridge::remove_relayer(Origin::root(), RELAYER_B),
					ChainbridgeError::<Runtime>::RelayerInvalid
				);
				assert_eq!(ChainBridge::relayer_count(), 2);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::RelayerAdded(RELAYER_A)),
					Event::ChainBridge(ChainbridgeEvent::RelayerAdded(RELAYER_B)),
					Event::ChainBridge(ChainbridgeEvent::RelayerAdded(RELAYER_C)),
					Event::ChainBridge(ChainbridgeEvent::RelayerRemoved(RELAYER_B)),
				]);
			})
		}

		fn make_proposal(r: Vec<u8>) -> Call {
			Call::System(system::Call::remark { remark: r })
		}

		#[test]
		fn create_sucessful_proposal() {
			let src_id = 1;
			let r_id = derive_resource_id(src_id, b"remark");

			new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
				let prop_id = 1;
				let proposal = make_proposal(vec![10]);

				// Create proposal (& vote)
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Second relayer votes against
				assert_ok!(ChainBridge::reject_proposal(
					Origin::signed(RELAYER_B),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![RELAYER_B],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Third relayer votes in favour
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_C),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A, RELAYER_C],
					votes_against: vec![RELAYER_B],
					status: ProposalStatus::Approved,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_A)),
					Event::ChainBridge(ChainbridgeEvent::VoteAgainst(src_id, prop_id, RELAYER_B)),
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_C)),
					Event::ChainBridge(ChainbridgeEvent::ProposalApproved(src_id, prop_id)),
					Event::ChainBridge(ChainbridgeEvent::ProposalSucceeded(src_id, prop_id)),
				]);
			})
		}

		#[test]
		fn create_unsucessful_proposal() {
			let src_id = 1;
			let r_id = derive_resource_id(src_id, b"transfer");

			new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
				let prop_id = 1;
				let proposal = make_proposal(vec![11]);

				// Create proposal (& vote)
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Second relayer votes against
				assert_ok!(ChainBridge::reject_proposal(
					Origin::signed(RELAYER_B),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![RELAYER_B],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Third relayer votes against
				assert_ok!(ChainBridge::reject_proposal(
					Origin::signed(RELAYER_C),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![RELAYER_B, RELAYER_C],
					status: ProposalStatus::Rejected,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				assert_eq!(Balances::free_balance(RELAYER_B), 0);
				assert_eq!(
					Balances::free_balance(ChainBridge::account_id()),
					ENDOWED_BALANCE
				);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_A)),
					Event::ChainBridge(ChainbridgeEvent::VoteAgainst(src_id, prop_id, RELAYER_B)),
					Event::ChainBridge(ChainbridgeEvent::VoteAgainst(src_id, prop_id, RELAYER_C)),
					Event::ChainBridge(ChainbridgeEvent::ProposalRejected(src_id, prop_id)),
				]);
			})
		}

		#[test]
		fn execute_after_threshold_change() {
			let src_id = 1;
			let r_id = derive_resource_id(src_id, b"transfer");

			new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
				let prop_id = 1;
				let proposal = make_proposal(vec![11]);

				// Create proposal (& vote)
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Change threshold
				assert_ok!(ChainBridge::set_threshold(Origin::root(), 1));

				// Attempt to execute
				assert_ok!(ChainBridge::eval_vote_state(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					Box::new(proposal.clone())
				));

				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Approved,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				assert_eq!(Balances::free_balance(RELAYER_B), 0);
				assert_eq!(
					Balances::free_balance(ChainBridge::account_id()),
					ENDOWED_BALANCE
				);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_A)),
					Event::ChainBridge(ChainbridgeEvent::RelayerThresholdChanged(1)),
					Event::ChainBridge(ChainbridgeEvent::ProposalApproved(src_id, prop_id)),
					Event::ChainBridge(ChainbridgeEvent::ProposalSucceeded(src_id, prop_id)),
				]);
			})
		}

		#[test]
		fn proposal_expires() {
			let src_id = 1;
			let r_id = derive_resource_id(src_id, b"remark");

			new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
				let prop_id = 1;
				let proposal = make_proposal(vec![10]);

				// Create proposal (& vote)
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Increment enough blocks such that now == expiry
				System::set_block_number(ProposalLifetime::get() + 1);

				// Attempt to submit a vote should fail
				assert_noop!(
					ChainBridge::reject_proposal(
						Origin::signed(RELAYER_B),
						prop_id,
						src_id,
						r_id,
						Box::new(proposal.clone())
					),
					ChainbridgeError::<Runtime>::ProposalExpired
				);

				// Proposal state should remain unchanged
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// eval_vote_state should have no effect
				assert_noop!(
					ChainBridge::eval_vote_state(
						Origin::signed(RELAYER_C),
						prop_id,
						src_id,
						Box::new(proposal.clone())
					),
					ChainbridgeError::<Runtime>::ProposalExpired
				);
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				para_assert_events(vec![Event::ChainBridge(ChainbridgeEvent::VoteFor(
					src_id, prop_id, RELAYER_A,
				))]);
			})
		}

		// ************** origin bridge-transfer tests *****************
		fn make_transfer_proposal(dest: Vec<u8>, src_id: u8, amount: Balance) -> Call {
			let resource_id = ChainBridge::gen_pha_rid(src_id);
			Call::ChainBridge(crate::chainbridge::Call::handle_fungible_transfer {
				dest,
				amount: amount.into(),
				rid: resource_id,
			})
		}

		#[test]
		fn transfer_assets_not_registered() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain = 2;
				let bridge_asset_location = MultiLocation::new(
					1,
					X4(
						Parachain(2004),
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(0),
						GeneralKey(b"an asset".to_vec()),
					),
				);
				let amount: Balance = 100;
				let recipient = vec![99];

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), dest_chain));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 2, dest_chain));

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_noop!(
					bridge_impl.transfer_fungible(
						ALICE.into(),
						(Concrete(bridge_asset_location), Fungible(amount)).into(),
						MultiLocation::new(
							0,
							X3(
								GeneralKey(b"cb".to_vec()),
								GeneralIndex(dest_chain.into()),
								GeneralKey(recipient)
							)
						),
						None,
					),
					// Can not pass can_deposit_asset check if assets hasn't been registered.
					ChainbridgeError::<Runtime>::CannotDepositAsset
				);
			})
		}

		#[test]
		fn transfer_assets_insufficient_balance() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain = 2;
				let bridge_asset_location = MultiLocation::new(
					1,
					X4(
						Parachain(2004),
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(0),
						GeneralKey(b"an asset".to_vec()),
					),
				);
				let amount: Balance = 100;
				let recipient = vec![99];

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), dest_chain));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 2, dest_chain));

				// Register asset, id = 0
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					bridge_asset_location.clone().into(),
					0,
					AssetProperties {
						name: b"BridgeAsset".to_vec(),
						symbol: b"BA".to_vec(),
						decimals: 12,
					},
				));

				// Setup solo chain for this asset
				assert_ok!(AssetsRegistry::force_enable_chainbridge(
					Origin::root(),
					0,
					dest_chain,
					true,
					Box::new(Vec::new()),
				));

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				// After registered, free balance of ALICE is 0
				assert_noop!(
					bridge_impl.transfer_fungible(
						ALICE.into(),
						(Concrete(bridge_asset_location), Fungible(amount)).into(),
						MultiLocation::new(
							0,
							X3(
								GeneralKey(b"cb".to_vec()),
								GeneralIndex(dest_chain.into()),
								GeneralKey(recipient)
							)
						),
						None,
					),
					ChainbridgeError::<Runtime>::InsufficientBalance
				);
			})
		}

		#[test]
		fn transfer_assets_to_nonreserve() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain: u8 = 2;
				let dest_reserve_location: MultiLocation = (
					0,
					X2(
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(dest_chain.into()),
					),
				)
					.into();
				let bridge_asset_location = SoloChain0AssetLocation::get();
				let amount: Balance = 100;
				let recipient = vec![99];

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), dest_chain));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 2, dest_chain));

				// Register asset, id = 0
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					bridge_asset_location.clone().into(),
					0,
					AssetProperties {
						name: b"BridgeAsset".to_vec(),
						symbol: b"BA".to_vec(),
						decimals: 12,
					},
				));

				// Setup solo chain for this asset
				assert_ok!(AssetsRegistry::force_enable_chainbridge(
					Origin::root(),
					0,
					dest_chain,
					true,
					Box::new(Vec::new()),
				));

				// Mint some token to ALICE
				assert_ok!(Assets::mint(
					Origin::signed(ASSETS_REGISTRY_ID.into_account()),
					0,
					ALICE,
					amount * 2
				));
				assert_eq!(Assets::balance(0, &ALICE), amount * 2);

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_ok!(bridge_impl.transfer_fungible(
					ALICE.into(),
					(Concrete(bridge_asset_location), Fungible(amount)).into(),
					MultiLocation::new(
						0,
						X3(
							GeneralKey(b"cb".to_vec()),
							GeneralIndex(dest_chain.into()),
							GeneralKey(recipient)
						)
					),
					None,
				));

				// Withdraw amount from ALICE
				assert_eq!(Assets::balance(0, &ALICE), amount);
				assert_eq!(Assets::balance(0, &TREASURY::get()), 2);

				// The asset's reserve chain is 0, dest chain is 2,
				// so will save (amount - fee) asset into reserve account of dest chain,
				assert_eq!(
					Assets::balance(0, &dest_reserve_location.into_account().into()),
					amount - 2
				);
			})
		}

		#[test]
		fn transfer_assets_to_reserve() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain: u8 = 2;
				let dest_reserve_location: MultiLocation = (
					0,
					X2(
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(dest_chain.into()),
					),
				)
					.into();
				let bridge_asset_location = SoloChain2AssetLocation::get();
				let amount: Balance = 100;
				let recipient = vec![99];

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), dest_chain));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 2, dest_chain));

				// Register asset, id = 0
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					bridge_asset_location.clone().into(),
					0,
					AssetProperties {
						name: b"BridgeAsset".to_vec(),
						symbol: b"BA".to_vec(),
						decimals: 12,
					},
				));

				// Setup solo chain for this asset
				assert_ok!(AssetsRegistry::force_enable_chainbridge(
					Origin::root(),
					0,
					dest_chain,
					true,
					Box::new(Vec::new()),
				));

				// Mint some token to ALICE
				assert_ok!(Assets::mint(
					Origin::signed(ASSETS_REGISTRY_ID.into_account()),
					0,
					ALICE,
					amount * 2
				));
				assert_eq!(Assets::balance(0, &ALICE), amount * 2);

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_ok!(bridge_impl.transfer_fungible(
					ALICE.into(),
					(Concrete(bridge_asset_location), Fungible(amount)).into(),
					MultiLocation::new(
						0,
						X3(
							GeneralKey(b"cb".to_vec()),
							GeneralIndex(dest_chain.into()),
							GeneralKey(recipient)
						)
					),
					None,
				));

				// Withdraw amount from ALICE
				assert_eq!(Assets::balance(0, &ALICE), amount);
				// Rate of PHA and SoloChain2AssetLocation accoate asset is 2:1
				assert_eq!(Assets::balance(0, &TREASURY::get()), 4);

				// The asset's reserve chain is 2, dest chain is 2,
				// so assets just be burned from sender
				assert_eq!(
					Assets::balance(0, &dest_reserve_location.into_account().into()),
					0
				);
			})
		}

		#[test]
		fn transfer_native() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain = 0;
				let resource_id = ChainBridge::gen_pha_rid(dest_chain);
				let free_balance: u128 = Balances::free_balance(RELAYER_A).into();
				let amount: Balance = 100;
				let recipient = vec![99];

				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), dest_chain));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 2, dest_chain));

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_noop!(
					bridge_impl.transfer_fungible(
						RELAYER_A.into(),
						(
							Concrete(MultiLocation::new(0, Here)),
							Fungible(free_balance + 1),
						)
							.into(),
						MultiLocation::new(
							0,
							X3(
								GeneralKey(b"cb".to_vec()),
								GeneralIndex(dest_chain.into()),
								GeneralKey(recipient.clone())
							)
						),
						None,
					),
					ChainbridgeError::<Runtime>::InsufficientBalance
				);

				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				assert_ok!(bridge_impl.transfer_fungible(
					RELAYER_A.into(),
					(Concrete(MultiLocation::new(0, Here)), Fungible(amount)).into(),
					MultiLocation::new(
						0,
						X3(
							GeneralKey(b"cb".to_vec()),
							GeneralIndex(dest_chain.into()),
							GeneralKey(recipient.clone())
						)
					),
					None,
				));

				para_expect_event(ChainbridgeEvent::FungibleTransfer(
					dest_chain,
					1,
					resource_id,
					(amount - 2).into(),
					recipient,
				));

				assert_eq!(
					Balances::free_balance(&ChainBridge::account_id()),
					ENDOWED_BALANCE + amount - 2
				)
			})
		}

		#[test]
		fn simulate_transfer_pha_from_solochain() {
			TestNet::reset();

			ParaA::execute_with(|| {
				// Check inital state
				let bridge_account = ChainBridge::account_id();
				let src_chainid = 0;
				let resource_id = ChainBridge::gen_pha_rid(src_chainid);
				assert_eq!(Balances::free_balance(&bridge_account), ENDOWED_BALANCE);
				let relayer_location = MultiLocation::new(
					0,
					X1(AccountId32 {
						network: NetworkId::Any,
						id: RELAYER_A.into(),
					}),
				);

				// Transfer and check result
				assert_ok!(ChainBridge::handle_fungible_transfer(
					Origin::signed(MODULE_ID.into_account()),
					relayer_location.encode(),
					10,
					resource_id,
				));
				assert_eq!(
					Balances::free_balance(&bridge_account),
					ENDOWED_BALANCE - 10
				);
				assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

				para_assert_events(vec![
					// Withdraw from reserve account(for PHA, is bridge account)
					Event::Balances(pallet_balances::Event::Withdraw {
						who: ChainBridge::account_id(),
						amount: 10,
					}),
					Event::XTransfer(crate::xtransfer::Event::Withdrawn {
						what: (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: ChainBridge::account_id().into(),
						}
						.into(),
						memo: Vec::new(),
					}),
					// Deposit into recipient
					Event::Balances(pallet_balances::Event::Deposit {
						who: RELAYER_A,
						amount: 10,
					}),
					Event::XTransfer(crate::xtransfer::Event::Deposited {
						what: (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: RELAYER_A.into(),
						}
						.into(),
						memo: Vec::new(),
					}),
				]);
			})
		}

		#[test]
		fn simulate_transfer_solochainassets_from_reserve_to_local() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let src_chainid: u8 = 0;
				let bridge_asset_location = MultiLocation::new(
					1,
					X4(
						Parachain(2004),
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(src_chainid.into()),
						GeneralKey(b"an asset".to_vec()),
					),
				);
				let r_id: [u8; 32] = bridge_asset_location.clone().into_rid(src_chainid);
				let amount: Balance = 100;
				let alice_location = MultiLocation::new(
					0,
					X1(AccountId32 {
						network: NetworkId::Any,
						id: ALICE.into(),
					}),
				);

				let src_reserve_location: MultiLocation = (
					0,
					X2(
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(src_chainid.into()),
					),
				)
					.into();

				// Register asset, id = 0
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					bridge_asset_location.clone(),
					0,
					AssetProperties {
						name: b"BridgeAsset".to_vec(),
						symbol: b"BA".to_vec(),
						decimals: 12,
					},
				));

				// Setup solo chain for this asset
				assert_ok!(AssetsRegistry::force_enable_chainbridge(
					Origin::root(),
					0,
					src_chainid,
					true,
					Box::new(Vec::new()),
				));

				assert_eq!(Assets::balance(0, &ALICE), 0);

				// Transfer from asset reserve location, would mint asset into ALICE directly
				assert_ok!(ChainBridge::handle_fungible_transfer(
					Origin::signed(MODULE_ID.into_account()),
					alice_location.encode(),
					amount,
					r_id,
				));
				assert_eq!(Assets::balance(0, &ALICE), amount);
				assert_eq!(
					Assets::balance(0, &src_reserve_location.into_account().into()),
					0
				);

				para_assert_events(vec![
					// Mint asset
					Event::Assets(pallet_assets::Event::Issued {
						asset_id: 0,
						owner: ALICE,
						total_supply: amount,
					}),
					Event::XTransfer(crate::xtransfer::Event::Deposited {
						what: (Concrete(bridge_asset_location), Fungible(amount)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: ALICE.into(),
						}
						.into(),
						memo: Vec::new(),
					}),
				]);
			})
		}

		#[test]
		fn simulate_transfer_solochainassets_from_nonreserve_to_local() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let src_chainid: u8 = 0;
				let para_asset_location = MultiLocation::new(
					1,
					X2(
						Parachain(2000),
						GeneralKey(b"an asset from karura".to_vec()),
					),
				);
				let amount: Balance = 100;
				let alice_location = MultiLocation::new(
					0,
					X1(AccountId32 {
						network: NetworkId::Any,
						id: ALICE.into(),
					}),
				);
				let src_reserve_location: MultiLocation = (
					0,
					X2(
						GeneralKey(CB_ASSET_KEY.to_vec()),
						GeneralIndex(src_chainid.into()),
					),
				)
					.into();

				// Register asset, id = 0
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_asset_location.clone(),
					0,
					AssetProperties {
						name: b"ParaAsset".to_vec(),
						symbol: b"PA".to_vec(),
						decimals: 12,
					},
				));

				// Setup solo chain for this asset
				assert_ok!(AssetsRegistry::force_enable_chainbridge(
					Origin::root(),
					0,
					src_chainid,
					true,
					Box::new(Vec::new()),
				));

				assert_eq!(Assets::balance(0, &ALICE), 0);

				// Mint some token to reserve account, simulate the reserve pool
				assert_ok!(Assets::mint(
					Origin::signed(ASSETS_REGISTRY_ID.into_account()),
					0,
					src_reserve_location.clone().into_account().into(),
					amount * 2
				));
				assert_eq!(
					Assets::balance(0, &src_reserve_location.clone().into_account().into()),
					amount * 2
				);
				para_assert_events(vec![
					// Mint asset
					Event::Assets(pallet_assets::Event::Issued {
						asset_id: 0,
						owner: src_reserve_location.clone().into_account().into(),
						total_supply: amount * 2,
					}),
				]);

				// Transfer from nonreserve location of asset,
				// first: burn asset from source reserve account
				// second: mint asset into recipient
				assert_ok!(ChainBridge::handle_fungible_transfer(
					Origin::signed(MODULE_ID.into_account()),
					alice_location.encode(),
					amount,
					para_asset_location.clone().into_rid(src_chainid),
				));
				assert_eq!(Assets::balance(0, &ALICE), amount);
				assert_eq!(
					Assets::balance(0, &src_reserve_location.clone().into_account().into()),
					amount
				);

				para_assert_events(vec![
					// Burn asset
					Event::Assets(pallet_assets::Event::Burned {
						asset_id: 0,
						owner: src_reserve_location.clone().into_account().into(),
						balance: amount,
					}),
					Event::XTransfer(crate::xtransfer::Event::Withdrawn {
						what: (Concrete(para_asset_location.clone()), Fungible(amount)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: src_reserve_location.into_account().into(),
						}
						.into(),
						memo: Vec::new(),
					}),
					// Mint asset
					Event::Assets(pallet_assets::Event::Issued {
						asset_id: 0,
						owner: ALICE,
						total_supply: amount,
					}),
					Event::XTransfer(crate::xtransfer::Event::Deposited {
						what: (Concrete(para_asset_location), Fungible(amount)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: ALICE.into(),
						}
						.into(),
						memo: Vec::new(),
					}),
				]);
			})
		}

		#[test]
		fn create_successful_transfer_proposal() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let prop_id = 1;
				let src_id = 1;
				let r_id = ChainBridge::gen_pha_rid(src_id);
				let relayer_location = MultiLocation::new(
					0,
					X1(AccountId32 {
						network: NetworkId::Any,
						id: RELAYER_A.into(),
					}),
				);
				let proposal = make_transfer_proposal(relayer_location.encode(), src_id, 10);

				assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
				assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_C));
				assert_ok!(ChainBridge::whitelist_chain(Origin::root(), src_id));

				// Create proposal (& vote)
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = crate::chainbridge::ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![],
					status: crate::chainbridge::ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Second relayer votes against
				assert_ok!(ChainBridge::reject_proposal(
					Origin::signed(RELAYER_B),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = crate::chainbridge::ProposalVotes {
					votes_for: vec![RELAYER_A],
					votes_against: vec![RELAYER_B],
					status: crate::chainbridge::ProposalStatus::Initiated,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				// Third relayer votes in favour
				assert_ok!(ChainBridge::acknowledge_proposal(
					Origin::signed(RELAYER_C),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone())
				));
				let prop = ChainBridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
				let expected = crate::chainbridge::ProposalVotes {
					votes_for: vec![RELAYER_A, RELAYER_C],
					votes_against: vec![RELAYER_B],
					status: crate::chainbridge::ProposalStatus::Approved,
					expiry: ProposalLifetime::get() + 1,
				};
				assert_eq!(prop, expected);

				assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);
				assert_eq!(
					Balances::free_balance(ChainBridge::account_id()),
					ENDOWED_BALANCE - 10
				);

				para_assert_events(vec![
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_A)),
					Event::ChainBridge(ChainbridgeEvent::VoteAgainst(src_id, prop_id, RELAYER_B)),
					Event::ChainBridge(ChainbridgeEvent::VoteFor(src_id, prop_id, RELAYER_C)),
					Event::ChainBridge(ChainbridgeEvent::ProposalApproved(src_id, prop_id)),
					// Withdraw from reserve account(for PHA, is bridge account)
					Event::Balances(pallet_balances::Event::Withdraw {
						who: ChainBridge::account_id(),
						amount: 10,
					}),
					Event::XTransfer(crate::xtransfer::Event::Withdrawn {
						what: (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: ChainBridge::account_id().into(),
						}
						.into(),
						memo: Vec::new(),
					}),
					// Deposit into recipient
					Event::Balances(pallet_balances::Event::Deposit {
						who: RELAYER_A,
						amount: 10,
					}),
					Event::XTransfer(crate::xtransfer::Event::Deposited {
						what: (Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
						who: Junction::AccountId32 {
							network: NetworkId::Any,
							id: RELAYER_A.into(),
						}
						.into(),
						memo: Vec::new(),
					}),
					Event::ChainBridge(ChainbridgeEvent::ProposalSucceeded(src_id, prop_id)),
				]);
			})
		}

		#[test]
		fn test_get_fee() {
			TestNet::reset();

			ParaA::execute_with(|| {
				let dest_chain: u8 = 2;
				let test_asset_location = MultiLocation::new(1, X1(GeneralKey(b"test".to_vec())));
				assert_ok!(ChainBridge::update_fee(Origin::root(), 2, 0, dest_chain));

				// Register asset, decimals: 18, rate with pha: 1 : 1
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					SoloChain0AssetLocation::get().into(),
					0,
					AssetProperties {
						name: b"BridgeAsset".to_vec(),
						symbol: b"BA".to_vec(),
						decimals: 18,
					},
				));
				// Register asset, decimals: 12, rate with pha: 1 : 2
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					SoloChain2AssetLocation::get().into(),
					1,
					AssetProperties {
						name: b"AnotherBridgeAsset".to_vec(),
						symbol: b"ABA".to_vec(),
						decimals: 12,
					},
				));

				// Register asset, decimals: 12, not set as fee payment
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					test_asset_location.clone().into(),
					2,
					AssetProperties {
						name: b"TestAsset".to_vec(),
						symbol: b"TA".to_vec(),
						decimals: 12,
					},
				));

				let asset0: MultiAsset = (
					SoloChain0AssetLocation::get(),
					Fungible(100_000_000_000_000_000_000),
				)
					.into();
				let asset2: MultiAsset = (
					SoloChain2AssetLocation::get(),
					Fungible(100_000_000_000_000),
				)
					.into();
				let test_asset: MultiAsset = (test_asset_location, Fungible(100)).into();

				// Test asset not configured as fee payment in trader
				assert_eq!(ChainBridge::get_fee(dest_chain, &test_asset), None);
				// Fee in pha is 2, decimal of balance is not set so will be set as 12 when calculating fee,
				// asset 0 decimals is 18, and rate with pha is 1:1
				// Final fee in asset 0 is 2_000_000
				assert_eq!(ChainBridge::get_fee(dest_chain, &asset0), Some(2_000_000),);
				// Fee in pha is 2, decimal of balance is not set so will be set as 12 when calculating fee,
				// asset 2 decimals is 12, and rate with pha is 2:1
				// Final fee in asset 2 is 4
				assert_eq!(ChainBridge::get_fee(dest_chain, &asset2), Some(4),);
			})
		}
	}
}
