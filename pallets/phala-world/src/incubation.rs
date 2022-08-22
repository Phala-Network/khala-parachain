//! Phala World Incubation Pallet

pub use crate::pallet_pw_nft_sale;
pub use crate::traits::{
	primitives::*, CareerType, FoodInfo, RaceType, RarityType, ShellPartInfo, ShellPartShape,
	ShellPartType, ShellSubPartInfo,
};
use codec::Decode;
use frame_support::{
	ensure,
	pallet_prelude::Get,
	traits::{tokens::nonfungibles::InspectEnumerable, UnixTime},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet_rmrk_core::types::*;
pub use pallet_rmrk_market;
use rmrk_traits::{primitives::*, Nft, Property};
use sp_runtime::DispatchResult;
use sp_std::vec::Vec;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::Origin;

	pub type ShellPartInfoOf<T> = ShellPartInfo<
		BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
		BoundedVec<
			ShellSubPartInfo<BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>>,
			<T as pallet_rmrk_core::Config>::PartsLimit,
		>,
	>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_rmrk_core::Config + pallet_pw_nft_sale::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Amount of food per Era.
		#[pallet::constant]
		type FoodPerEra: Get<u32>;
		/// Max food to feed your own Origin of Shell.
		#[pallet::constant]
		type MaxFoodFeedSelf: Get<u8>;
		/// Duration of incubation process.
		#[pallet::constant]
		type IncubationDurationSec: Get<u64>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Info on Origin of Shells that the Owner has fed.
	#[pallet::storage]
	#[pallet::getter(fn food_by_owners)]
	pub type FoodByOwners<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		FoodInfo<BoundedVec<(CollectionId, NftId), <T as Config>::FoodPerEra>>,
	>;

	/// Total food fed to an Origin of Shell per Era.
	#[pallet::storage]
	#[pallet::getter(fn origin_of_shell_food_stats)]
	pub type OriginOfShellFoodStats<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraId,
		Blake2_128Concat,
		(CollectionId, NftId),
		u32,
		ValueQuery,
	>;

	/// Official hatch time for all Origin of Shells.
	#[pallet::storage]
	#[pallet::getter(fn official_hatch_time)]
	pub type OfficialHatchTime<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// A bool value to determine if accounts can start incubation of Origin of Shells.
	#[pallet::storage]
	#[pallet::getter(fn can_start_incubation)]
	pub type CanStartIncubation<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// A bool value to determine if an Origin of Shell has started the incubation process.
	#[pallet::storage]
	#[pallet::getter(fn has_origin_of_shell_started_incubation)]
	pub type HasOriginOfShellStartedIncubation<T: Config> =
		StorageMap<_, Blake2_128Concat, (CollectionId, NftId), bool, ValueQuery>;

	/// Collection ID of the Shell NFT.
	#[pallet::storage]
	#[pallet::getter(fn shell_collection_id)]
	pub type ShellCollectionId<T: Config> = StorageValue<_, CollectionId>;

	/// Collection ID of the Shell Parts NFTs.
	#[pallet::storage]
	#[pallet::getter(fn shell_parts_collection_id)]
	pub type ShellPartsCollectionId<T: Config> = StorageValue<_, CollectionId>;

	/// Storage of an account's selected parts during the incubation process.
	#[pallet::storage]
	#[pallet::getter(fn origin_of_shells_chosen_parts)]
	pub type OriginOfShellsChosenParts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(CollectionId, NftId),
		Blake2_128Concat,
		BoundedVec<u8, T::StringLimit>,
		ShellPartInfoOf<T>,
	>;

	/// Number of special parts chosen for each Origin of Shell
	#[pallet::storage]
	#[pallet::getter(fn origin_of_shells_special_parts_count)]
	pub type OriginOfShellsSpecialPartsCount<T: Config> =
		StorageMap<_, Blake2_128Concat, (CollectionId, NftId), u16, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// CanStartIncubation status changed and set official hatch time.
		CanStartIncubationStatusChanged {
			status: bool,
			start_time: u64,
			official_hatch_time: u64,
		},
		/// Origin of Shell owner has initiated the incubation sequence.
		StartedIncubation {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
			start_time: u64,
			hatch_time: u64,
		},
		/// Origin of Shell received food from an account.
		OriginOfShellReceivedFood {
			collection_id: CollectionId,
			nft_id: NftId,
			sender: T::AccountId,
		},
		/// Origin of Shell updated chosen parts.
		OriginOfShellChosenPartsUpdated {
			collection_id: CollectionId,
			nft_id: NftId,
			shell_part: BoundedVec<u8, T::StringLimit>,
			old_chosen_part: Option<ShellPartInfoOf<T>>,
			new_chosen_part: ShellPartInfoOf<T>,
		},
		/// Shell Collection ID is set.
		ShellCollectionIdSet { collection_id: CollectionId },
		/// Shell Parts Collection ID is set.
		ShellPartsCollectionIdSet { collection_id: CollectionId },
		/// Shell has been awakened from an origin_of_shell being hatched and burned.
		ShellAwakened {
			shell_collection_id: CollectionId,
			shell_nft_id: NftId,
			rarity: RarityType,
			career: CareerType,
			race: RaceType,
			generation_id: GenerationId,
			origin_of_shell_collection_id: CollectionId,
			origin_of_shell_nft_id: NftId,
			owner: T::AccountId,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		StartIncubationNotAvailable,
		HatchingInProgress,
		CannotHatchOriginOfShell,
		CannotSendFoodToOriginOfShell,
		CannotSetOriginOfShellChosenParts,
		MaxFoodFedLimitReached,
		AlreadySentFoodTwice,
		NoFoodAvailable,
		NotOwner,
		NoPermission,
		WrongCollectionId,
		ShellCollectionIdAlreadySet,
		ShellPartsCollectionIdAlreadySet,
		ShellCollectionIdNotSet,
		ShellPartsCollectionIdNotSet,
		RaceNotDetected,
		CareerNotDetected,
		RarityTypeNotDetected,
		GenerationNotDetected,
		FoodInfoUpdateError,
		ChosenPartsDataCorrupted,
		MaxSpecialPartsLimitReached,
		MissingShellSubParts,
		MissingShellPartMetadata,
		ExpectedShellSubPartType,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
	{
		/// Once users have received their origin_of_shells and the start incubation event has been
		/// triggered, they can start the incubation process and a timer will start for the
		/// origin_of_shell to awaken at a designated time. Origin of Shells can reduce their time
		/// by being in the top 10 of origin_of_shell's fed per era.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic starting the incubation process
		/// - collection_id: The collection id of the Origin of Shell RMRK NFT
		/// - nft_id: The NFT id of the Origin of Shell RMRK NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn start_incubation(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if IncubationPhase is enabled
			ensure!(
				CanStartIncubation::<T>::get(),
				Error::<T>::StartIncubationNotAvailable
			);
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id),
				Error::<T>::WrongCollectionId
			);
			// Ensure sender is owner
			ensure!(
				Self::is_owner(&sender, collection_id, nft_id),
				Error::<T>::NotOwner
			);
			// Ensure incubation process hasn't been started already
			ensure!(
				!HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id)),
				Error::<T>::HatchingInProgress
			);
			// Get time to start hatching process
			let start_time = T::Time::now().as_secs();
			let hatch_time = OfficialHatchTime::<T>::get();
			// Update Hatch Time storage
			HasOriginOfShellStartedIncubation::<T>::insert((collection_id, nft_id), true);

			Self::deposit_event(Event::StartedIncubation {
				owner: sender,
				collection_id,
				nft_id,
				start_time,
				hatch_time,
			});

			Ok(())
		}

		/// Feed another origin_of_shell to the current origin_of_shell being incubated. This will
		/// reduce the time left to incubation if the origin_of_shell is in the top 10 of
		/// origin_of_shells fed that era.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic feeding the origin_of_shell
		/// - collection_id: The collection id of the Origin of Shell RMRK NFT
		/// - nft_id: The NFT id of the Origin of Shell RMRK NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn feed_origin_of_shell(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if Incubation Phase has started
			ensure!(
				CanStartIncubation::<T>::get(),
				Error::<T>::StartIncubationNotAvailable
			);
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id),
				Error::<T>::WrongCollectionId
			);
			// Ensure that Origin of Shell exists and is not past the hatch time
			ensure!(
				HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id))
					&& !Self::can_hatch(),
				Error::<T>::CannotSendFoodToOriginOfShell
			);
			// Check if account owns an Origin of Shell NFT
			ensure!(
				pallet_uniques::pallet::Pallet::<T>::owned_in_collection(&collection_id, &sender)
					.count() > 0,
				Error::<T>::NoPermission
			);
			// Get Current Era
			let current_era = pallet_pw_nft_sale::Era::<T>::get();
			// Get number of times fed this era
			let num_of_times_fed =
				OriginOfShellFoodStats::<T>::get(current_era, (collection_id, nft_id));

			// Update account FoodInfo if not updated or create new FoodInfo for account's first
			// feeding
			FoodByOwners::<T>::try_mutate(&sender, |food_info| -> DispatchResult {
				let mut new_food_info = match food_info {
					None => Self::get_new_food_info(current_era),
					Some(food_info) if current_era > food_info.era => {
						Self::get_new_food_info(current_era)
					}
					Some(food_info) => {
						// Ensure sender hasn't fed the Origin of Shell 2 times
						ensure!(
							food_info
								.origin_of_shells_fed
								.iter()
								.filter(|&nft| *nft == (collection_id, nft_id))
								.count() < T::MaxFoodFeedSelf::get().into(),
							Error::<T>::AlreadySentFoodTwice
						);
						food_info.clone()
					}
				};
				ensure!(
					new_food_info
						.origin_of_shells_fed
						.try_push((collection_id, nft_id))
						.is_ok(),
					Error::<T>::FoodInfoUpdateError
				);
				*food_info = Some(new_food_info);

				Ok(())
			})?;

			// Update the Origin of Shell food stats
			OriginOfShellFoodStats::<T>::insert(
				current_era,
				(collection_id, nft_id),
				num_of_times_fed + 1,
			);

			Self::deposit_event(Event::OriginOfShellReceivedFood {
				collection_id,
				nft_id,
				sender,
			});

			Ok(())
		}

		/// Hatch the origin_of_shell that is currently being hatched. This will trigger the end of
		/// the incubation process and the origin_of_shell will be burned. After burning, the user
		/// will receive the awakened Shell RMRK NFT.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic incubation the origin_of_shell
		/// - collection_id: The collection id of the Origin of Shell RMRK NFT
		/// - nft_id: The NFT id of the Origin of Shell RMRK NFT
		/// - metadata: File resource URI in decentralized storage
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn hatch_origin_of_shell(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			metadata: BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			pallet_pw_nft_sale::pallet::Pallet::<T>::ensure_overlord(&sender)?;
			// Check if Incubation Phase has started
			ensure!(
				CanStartIncubation::<T>::get(),
				Error::<T>::StartIncubationNotAvailable
			);
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id),
				Error::<T>::WrongCollectionId
			);
			// Get owner of the Origin of Shell NFT
			let (owner, _) =
				pallet_rmrk_core::Pallet::<T>::lookup_root_owner(collection_id, nft_id)?;
			// Check if the incubation has started and the official hatch time has been met
			ensure!(
				HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id))
					&& Self::can_hatch(),
				Error::<T>::CannotHatchOriginOfShell
			);
			// Check if Shell Collection ID is set
			let shell_collection_id = Self::get_shell_collection_id()?;
			// Get race, key and Rarity Type before burning origin of shell NFT
			let race_key = pallet_pw_nft_sale::pallet::Pallet::<T>::to_boundedvec_key("race")?;
			let career_key = pallet_pw_nft_sale::pallet::Pallet::<T>::to_boundedvec_key("career")?;
			let rarity_type_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("rarity")?;
			let generation_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("generation")?;
			let race =
				pallet_rmrk_core::Properties::<T>::get((collection_id, Some(nft_id), &race_key))
					.ok_or(Error::<T>::RaceNotDetected)?;
			let career =
				pallet_rmrk_core::Properties::<T>::get((collection_id, Some(nft_id), &career_key))
					.ok_or(Error::<T>::CareerNotDetected)?;
			let rarity_type_value = pallet_rmrk_core::Properties::<T>::get((
				collection_id,
				Some(nft_id),
				&rarity_type_key,
			))
			.ok_or(Error::<T>::RarityTypeNotDetected)?;
			let generation = pallet_rmrk_core::Properties::<T>::get((
				collection_id,
				Some(nft_id),
				&generation_key,
			))
			.ok_or(Error::<T>::GenerationNotDetected)?;
			let race_type: RaceType =
				Decode::decode(&mut race.as_slice()).expect("[race] should not fail");
			let career_type: CareerType =
				Decode::decode(&mut career.as_slice()).expect("[career] should not fail");
			let rarity_type: RarityType = Decode::decode(&mut rarity_type_value.as_slice())
				.expect("[rarity] should not fail");
			let generation_id: GenerationId =
				Decode::decode(&mut generation.as_slice()).expect("[generation] should not fail");
			// Get next expected Shell NFT ID
			let next_shell_nft_id =
				pallet_pw_nft_sale::Pallet::<T>::get_next_nft_id(shell_collection_id)?;
			// Burn Origin of Shell NFT then Mint Shell NFT
			pallet_rmrk_core::Pallet::<T>::burn_nft(
				Origin::<T>::Signed(owner.clone()).into(),
				collection_id,
				nft_id,
				1,
			)?;
			// Remove Properties from Uniques pallet
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				race_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				career_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				rarity_type_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				generation_key,
			)?;
			// Mint Shell NFT to Overlord to add properties and resource before sending to owner
			let (_, shell_nft_id) = pallet_rmrk_core::Pallet::<T>::nft_mint(
				owner.clone(),
				owner.clone(),
				next_shell_nft_id,
				shell_collection_id,
				None,
				None,
				metadata,
				true,
				None,
			)?;
			// Set Rarity Type, Race and Career properties for NFT
			pallet_pw_nft_sale::Pallet::<T>::set_nft_properties(
				shell_collection_id,
				shell_nft_id,
				rarity_type,
				race_type,
				career_type,
				generation_id,
			)?;

			Self::deposit_event(Event::ShellAwakened {
				shell_collection_id,
				shell_nft_id,
				rarity: rarity_type,
				career: career_type,
				race: race_type,
				generation_id,
				origin_of_shell_collection_id: collection_id,
				origin_of_shell_nft_id: nft_id,
				owner,
			});

			Ok(())
		}

		/// Hatch the origin_of_shell that is currently being hatched. This will trigger the end of
		/// the incubation process and the origin_of_shell will be burned. After burning, the user
		/// will receive the awakened Shell RMRK NFT and the nested NFT parts that renders the Shell
		/// NFT.
		///
		/// Parameters:
		/// - `origin`: The origin of the extrinsic incubation the origin_of_shell
		/// - `collection_id`: The collection id of the Origin of Shell RMRK NFT
		/// - `nft_id`: The NFT id of the Origin of Shell RMRK NFT
		/// - `shell_metadata`: File resource URI in decentralized storage for Shell NFT
		///	parts that render the Shell NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn hatch_origin_of_shell2(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			default_shell_metadata: BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			pallet_pw_nft_sale::pallet::Pallet::<T>::ensure_overlord(&sender)?;
			// Check if Incubation Phase has started
			ensure!(
				CanStartIncubation::<T>::get(),
				Error::<T>::StartIncubationNotAvailable
			);
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id),
				Error::<T>::WrongCollectionId
			);
			// Get owner of the Origin of Shell NFT
			let (owner, _) =
				pallet_rmrk_core::Pallet::<T>::lookup_root_owner(collection_id, nft_id)?;
			// Check if the incubation has started and the official hatch time has been met
			ensure!(
				HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id))
					&& Self::can_hatch(),
				Error::<T>::CannotHatchOriginOfShell
			);
			// Check if Shell Collection ID is set
			let shell_collection_id = Self::get_shell_collection_id()?;
			// Check if Shell Parts Collection ID is set
			let shell_parts_collection_id = Self::get_shell_parts_collection_id()?;
			// Get race, key and Rarity Type before burning origin of shell NFT
			let race_key = pallet_pw_nft_sale::pallet::Pallet::<T>::to_boundedvec_key("race")?;
			let career_key = pallet_pw_nft_sale::pallet::Pallet::<T>::to_boundedvec_key("career")?;
			let rarity_type_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("rarity")?;
			let generation_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("generation")?;
			let race =
				pallet_rmrk_core::Properties::<T>::get((collection_id, Some(nft_id), &race_key))
					.ok_or(Error::<T>::RaceNotDetected)?;
			let career =
				pallet_rmrk_core::Properties::<T>::get((collection_id, Some(nft_id), &career_key))
					.ok_or(Error::<T>::CareerNotDetected)?;
			let rarity_type_value = pallet_rmrk_core::Properties::<T>::get((
				collection_id,
				Some(nft_id),
				&rarity_type_key,
			))
			.ok_or(Error::<T>::RarityTypeNotDetected)?;
			let generation = pallet_rmrk_core::Properties::<T>::get((
				collection_id,
				Some(nft_id),
				&generation_key,
			))
			.ok_or(Error::<T>::GenerationNotDetected)?;
			let race_type: RaceType =
				Decode::decode(&mut race.as_slice()).expect("[race] should not fail");
			let career_type: CareerType =
				Decode::decode(&mut career.as_slice()).expect("[career] should not fail");
			let rarity_type: RarityType = Decode::decode(&mut rarity_type_value.as_slice())
				.expect("[rarity] should not fail");
			let generation_id: GenerationId =
				Decode::decode(&mut generation.as_slice()).expect("[generation] should not fail");
			// Get next expected Shell NFT ID
			let next_shell_nft_id =
				pallet_pw_nft_sale::Pallet::<T>::get_next_nft_id(shell_collection_id)?;
			// Burn Origin of Shell NFT then Mint Shell NFT
			pallet_rmrk_core::Pallet::<T>::burn_nft(
				Origin::<T>::Signed(owner.clone()).into(),
				collection_id,
				nft_id,
				1,
			)?;
			// Remove Properties from Uniques pallet
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				race_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				career_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				rarity_type_key,
			)?;
			pallet_rmrk_core::Pallet::<T>::do_remove_property(
				collection_id,
				Some(nft_id),
				generation_key,
			)?;
			// Mint Shell NFT to Overlord to add properties and resource before sending to owner
			let (_, shell_nft_id) = pallet_rmrk_core::Pallet::<T>::nft_mint(
				owner.clone(),
				owner.clone(),
				next_shell_nft_id,
				shell_collection_id,
				None,
				None,
				default_shell_metadata,
				true,
				None,
			)?;
			// Set Rarity Type, Race and Career properties for NFT
			pallet_pw_nft_sale::Pallet::<T>::set_nft_properties(
				shell_collection_id,
				shell_nft_id,
				rarity_type,
				race_type,
				career_type,
				generation_id,
			)?;

			// Iterate through the chosen parts
			for (shell_part, chosen_part) in
				OriginOfShellsChosenParts::<T>::drain_prefix((collection_id, nft_id))
			{
				let special = chosen_part.special;
				let part_type = chosen_part.part_type;
				match part_type {
					ShellPartType::ComposablePart => {
						let shell_sub_parts = chosen_part
							.sub_parts
							.ok_or(Error::<T>::MissingShellSubParts)?;
						// The Shell part is a composable and made up of Shell Sub-Parts
						let (_, shell_part_nft_id) = Self::do_mint_shell_part_nft(
							owner.clone(),
							chosen_part.name,
							chosen_part.shape,
							shell_part,
							generation_id,
							special,
							race_type,
							career_type,
							Default::default(),
							shell_parts_collection_id,
							shell_collection_id,
							shell_nft_id,
						)?;
						for shell_sub_part in shell_sub_parts {
							ensure!(
								shell_sub_part.part_type == ShellPartType::SubPart,
								Error::<T>::ExpectedShellSubPartType
							);
							Self::do_mint_shell_sub_part_nft(
								owner.clone(),
								shell_sub_part.name,
								shell_sub_part.shape,
								generation_id,
								special,
								race_type,
								career_type,
								shell_sub_part.metadata,
								shell_parts_collection_id,
								shell_parts_collection_id,
								shell_part_nft_id,
							)?;
						}
					}
					ShellPartType::BasicPart => {
						// A basic Shell part
						let shell_part_metadata = chosen_part
							.metadata
							.ok_or(Error::<T>::MissingShellPartMetadata)?;
						let (_, shell_part_nft_id) = Self::do_mint_shell_part_nft(
							owner.clone(),
							chosen_part.name,
							chosen_part.shape,
							shell_part,
							generation_id,
							special,
							race_type,
							career_type,
							shell_part_metadata,
							shell_parts_collection_id,
							shell_collection_id,
							shell_nft_id,
						)?;
					}
					_ => return Err(Error::<T>::ChosenPartsDataCorrupted.into()),
				}
			}

			Self::deposit_event(Event::ShellAwakened {
				shell_collection_id,
				shell_nft_id,
				rarity: rarity_type,
				career: career_type,
				race: race_type,
				generation_id,
				origin_of_shell_collection_id: collection_id,
				origin_of_shell_nft_id: nft_id,
				owner,
			});

			Ok(())
		}

		/// Privileged function to enable incubation phase for accounts to start the incubation
		/// process for their Origin of Shells.
		///
		/// Parameters:
		/// `origin`: Expected to be the `Overlord` account
		/// `status`: `bool` value to set for the status in storage
		#[pallet::weight(0)]
		pub fn set_can_start_incubation_status(
			origin: OriginFor<T>,
			status: bool,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account is the sender
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// Get official hatch time to set in storage
			let start_time = T::Time::now().as_secs();
			let incubation_duration = T::IncubationDurationSec::get();
			let official_hatch_time = start_time + incubation_duration;
			// Set official hatch time
			<OfficialHatchTime<T>>::put(official_hatch_time);
			// Set status in storage
			<CanStartIncubation<T>>::put(status);

			Self::deposit_event(Event::CanStartIncubationStatusChanged {
				status,
				start_time,
				official_hatch_time,
			});

			Ok(Pays::No.into())
		}

		/// Privileged function to set the collection id for the Awakened Shell collection.
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the Shell Collection ID
		/// - `collection_id` - Collection ID of the Shell Collection
		#[pallet::weight(0)]
		pub fn set_shell_collection_id(
			origin: OriginFor<T>,
			collection_id: CollectionId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// If Spirit Collection ID is not None then the collection ID was already set
			ensure!(
				ShellCollectionId::<T>::get().is_none(),
				Error::<T>::ShellCollectionIdAlreadySet
			);
			<ShellCollectionId<T>>::put(collection_id);

			Self::deposit_event(Event::ShellCollectionIdSet { collection_id });

			Ok(Pays::No.into())
		}

		/// Privileged function to set the collection id for the Shell Parts collection.
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the Shell Parts Collection ID
		/// - `collection_id` - Collection ID of the Shell Parts Collection
		#[pallet::weight(0)]
		pub fn set_shell_parts_collection_id(
			origin: OriginFor<T>,
			collection_id: CollectionId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// Shell Parts Collection ID is None or else the collection ID was already set
			ensure!(
				ShellPartsCollectionId::<T>::get().is_none(),
				Error::<T>::ShellPartsCollectionIdAlreadySet
			);
			<ShellPartsCollectionId<T>>::put(collection_id);

			Self::deposit_event(Event::ShellPartsCollectionIdSet { collection_id });

			Ok(Pays::No.into())
		}

		/// Privileged function to set the part chosen by an account for a specific Origin of Shell.
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the chosen part to the Origin of Shell
		/// - `collection_id` - Collection ID of Origin of Shell
		/// - `nft_id` - NFT ID of the Origin of Shell
		#[pallet::weight(0)]
		pub fn set_origin_of_shell_chosen_parts(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			shell_part: BoundedVec<u8, T::StringLimit>,
			chosen_part: ShellPartInfoOf<T>,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id),
				Error::<T>::WrongCollectionId
			);
			// Ensure the incubation process has started before setting chosen parts
			ensure!(
				HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id)),
				Error::<T>::CannotSetOriginOfShellChosenParts
			);

			let rarity_type_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("rarity")?;
			let rarity_type_value = pallet_rmrk_core::Properties::<T>::get((
				collection_id,
				Some(nft_id),
				&rarity_type_key,
			))
			.ok_or(Error::<T>::RarityTypeNotDetected)?;
			let rarity_type: RarityType = Decode::decode(&mut rarity_type_value.as_slice())
				.expect("[rarity] should not fail");

			let is_special_part = chosen_part.special;

			let old_chosen_part =
				OriginOfShellsChosenParts::<T>::get((collection_id, nft_id), shell_part.clone())
					.clone();

			// If old chosen part is None or not a special part && is_special_part is true
			// then increment the special parts count. If old chosen part is a special part and the
			// the new chosen part is not then decrement the special parts count
			match old_chosen_part.clone() {
				Some(old_chosen_part_value) => {
					if old_chosen_part_value.special && !is_special_part {
						Self::decrement_origin_of_shell_special_parts_count(collection_id, nft_id);
					} else if !old_chosen_part_value.special && is_special_part {
						// Check if can add another special part
						Self::can_add_special_part(collection_id, nft_id, rarity_type)?;
						Self::increment_origin_of_shell_special_parts_count(collection_id, nft_id);
					}
				}
				None => {
					if is_special_part {
						// Check if can add another special part
						Self::can_add_special_part(collection_id, nft_id, rarity_type)?;
						Self::increment_origin_of_shell_special_parts_count(collection_id, nft_id);
					}
				}
			}

			// Update chosen parts storage
			OriginOfShellsChosenParts::<T>::insert(
				(collection_id, nft_id),
				shell_part.clone(),
				chosen_part.clone(),
			);

			Self::deposit_event(Event::OriginOfShellChosenPartsUpdated {
				collection_id,
				nft_id,
				shell_part,
				old_chosen_part,
				new_chosen_part: chosen_part,
			});

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	T: pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
{
	/// Helper function to ensure that the sender owns the origin of shell NFT.
	///
	/// Parameters:
	/// - `sender`: Sender to check if owns the NFT
	/// - `collection_id`: Collection ID of the NFT
	/// - `nft_id`: NFT ID of the NFT
	fn is_owner(sender: &T::AccountId, collection_id: CollectionId, nft_id: NftId) -> bool {
		if let Some(owner) = pallet_uniques::Pallet::<T>::owner(collection_id, nft_id) {
			sender == &owner
		} else {
			// No owner detected return false
			false
		}
	}

	/// Helper function to check the Collection ID matches Origin of Shell Collection ID.
	///
	/// Parameters:
	/// - `collection_id`: Collection ID to check
	fn is_origin_of_shell_collection_id(collection_id: CollectionId) -> bool {
		if let Some(origin_of_shell_collection_id) =
			pallet_pw_nft_sale::OriginOfShellCollectionId::<T>::get()
		{
			collection_id == origin_of_shell_collection_id
		} else {
			false
		}
	}

	/// Helper function to check if the Origin of Shell can hatch
	fn can_hatch() -> bool {
		let now = T::Time::now().as_secs();
		now > OfficialHatchTime::<T>::get()
	}

	/// Helper function to get collection id Shell collection
	fn get_shell_collection_id() -> Result<CollectionId, Error<T>> {
		let shell_collection_id =
			ShellCollectionId::<T>::get().ok_or(Error::<T>::ShellCollectionIdNotSet)?;
		Ok(shell_collection_id)
	}

	/// Helper function to get collection id Shell Parts collection
	fn get_shell_parts_collection_id() -> Result<CollectionId, Error<T>> {
		let shell_parts_collection_id =
			ShellPartsCollectionId::<T>::get().ok_or(Error::<T>::ShellPartsCollectionIdNotSet)?;
		Ok(shell_parts_collection_id)
	}

	/// Helper function to get new FoodOf<T> struct
	///
	/// Parameters:
	/// - `era`: The current Era of Phala World
	fn get_new_food_info(
		era: EraId,
	) -> FoodInfo<BoundedVec<(CollectionId, NftId), <T as Config>::FoodPerEra>> {
		FoodInfo {
			era,
			origin_of_shells_fed: Default::default(),
		}
	}

	/// Helper function to check if a special chosen part can be chosen based on the `Raritytype` of
	/// the Origin of Shell. Legendary have unlimited special parts, Magic has up to 5 special parts
	/// and Prime can have up to 2 special parts
	///
	/// Parameters:
	/// `collection_id`: Collection ID of the Origin of Shell
	/// `nft_id`: NFT ID of the Origin of Shell
	/// `rarity_type`: Rarity typeof the Origin of Shell
	fn can_add_special_part(
		collection_id: CollectionId,
		nft_id: NftId,
		rarity_type: RarityType,
	) -> DispatchResult {
		let current_special_parts_count =
			OriginOfShellsSpecialPartsCount::<T>::get((collection_id, nft_id));
		match rarity_type {
			RarityType::Legendary => Ok(()),
			RarityType::Magic => {
				if current_special_parts_count < 5 {
					Ok(())
				} else {
					return Err(Error::<T>::MaxSpecialPartsLimitReached.into());
				}
			}
			RarityType::Prime => {
				if current_special_parts_count < 2 {
					Ok(())
				} else {
					return Err(Error::<T>::MaxSpecialPartsLimitReached.into());
				}
			}
		}
	}

	/// Helper function to increment the special chosen parts for an Origin of Shell NFT
	///
	/// Parameters:
	/// - `collection_id`: Collection ID of the chosen parts
	/// - `nft_id`: NFT ID of the chosen parts
	fn increment_origin_of_shell_special_parts_count(collection_id: CollectionId, nft_id: NftId) {
		OriginOfShellsSpecialPartsCount::<T>::mutate(
			(collection_id, nft_id),
			|special_parts_count| {
				*special_parts_count += 1;
				*special_parts_count
			},
		);
	}

	/// Helper function to decrement the special chosen parts for an Origin of Shell NFT
	///
	/// Parameters:
	/// - `collection_id`: Collection ID of the chosen parts
	/// - `nft_id`: NFT ID of the chosen parts
	fn decrement_origin_of_shell_special_parts_count(collection_id: CollectionId, nft_id: NftId) {
		OriginOfShellsSpecialPartsCount::<T>::mutate(
			(collection_id, nft_id),
			|special_parts_count| {
				*special_parts_count -= 1;
				*special_parts_count
			},
		);
	}

	/// Helper function to mint a Top level Shell part. These are transferable Shell part NFTs.
	///
	/// Parameters:
	/// - `owner`: Root owner of the Shell part
	/// - `name`: Name of the Shell part
	/// - `shape`: Shell part shape of the Shell part
	/// - `generation`: Generation ID of the Shell Part
	/// - `rarity_type`: Rarity type of the Shell Part
	/// - `race_type`: Race type of the Shell Part
	/// - `career_type`: Career type of the Shell Part
	/// - `metadata`: Optional of metadata URI that will point to decentralized storage of the media
	/// file
	/// - `parent_nft_id`: NFT ID of the Shell NFT that owns the Shell Part
	fn do_mint_shell_part_nft(
		owner: T::AccountId,
		name: BoundedVec<u8, T::StringLimit>,
		shape: ShellPartShape,
		shell_part: BoundedVec<u8, T::StringLimit>,
		generation: GenerationId,
		special: bool,
		race_type: RaceType,
		career_type: CareerType,
		metadata: BoundedVec<u8, T::StringLimit>,
		shell_parts_collection_id: CollectionId,
		parent_collection_id: CollectionId,
		parent_nft_id: NftId,
	) -> Result<(CollectionId, NftId), Error<T>> {
		let name_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("name");
		let shell_part_key = pallet_pw_nft_sale::Pallet::<T>::to_boundedvec_key("shell_part");
		Ok((parent_collection_id, parent_nft_id))
	}

	/// Helper function to mint a Top level Shell part. These are non-transferable Shell sub-part NFTs
	/// that are owned by the parent Shell part NFT.
	///
	/// Parameters:
	/// - `owner`: Root owner of the Shell part
	/// - `name`: Name of the Shell part
	/// - `shape`: Shell part shape of the Shell part
	/// - `generation`: Generation ID of the Shell Part
	/// - `rarity_type`: Rarity type of the Shell Part
	/// - `race_type`: Race type of the Shell Part
	/// - `career_type`: Career type of the Shell Part
	/// - `metadata`: Optional of metadata URI that will point to decentralized storage of the media
	/// file
	/// - `parent_nft_id`: NFT ID of the Shell NFT that owns the Shell Part
	fn do_mint_shell_sub_part_nft(
		owner: T::AccountId,
		name: BoundedVec<u8, T::StringLimit>,
		shape: ShellPartShape,
		generation: GenerationId,
		special: bool,
		race_type: RaceType,
		career_type: CareerType,
		metadata: BoundedVec<u8, T::StringLimit>,
		shell_parts_collection_id: CollectionId,
		parent_collection_id: CollectionId,
		parent_nft_id: NftId,
	) -> DispatchResult {
		Ok(())
	}
}
