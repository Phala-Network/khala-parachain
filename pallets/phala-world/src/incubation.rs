//! Phala World Incubation Pallet

pub use crate::pallet_pw_nft_sale;
pub use crate::traits::{
	primitives::*, property_value, CareerType, FoodInfo, PartInfo, RaceType, RarityType,
	ShellPartInfo, ShellParts,
};
use alloc::{collections::BTreeMap, vec};
use codec::{alloc, Decode};
use frame_support::{
	ensure,
	pallet_prelude::Get,
	traits::{tokens::nonfungibles::InspectEnumerable, UnixTime},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*, Origin};
pub use pallet_rmrk_core::types::*;
pub use pallet_rmrk_market;
use rmrk_traits::{budget, primitives::*, Nft};
use sp_runtime::{DispatchError, Permill};
use sp_std::vec::Vec;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};

	pub type ShellPartInfoOf<T> = ShellPartInfo<
		BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
		BoundedVec<
			PartInfo<BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>>,
			<T as pallet_rmrk_core::Config>::PartsLimit,
		>,
	>;
	pub type ShellPartsOf<T> =
		ShellParts<BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>, ShellPartInfoOf<T>>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_rmrk_core::Config + pallet_pw_nft_sale::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
	pub struct Pallet<T>(_);

	/// Info on Origin of Shells that the Owner has fed and the number of food left to feed other
	/// Origin of Shells.
	#[pallet::storage]
	#[pallet::getter(fn food_by_owners)]
	pub type FoodByOwners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, FoodInfo>;

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
		StorageMap<_, Twox64Concat, (CollectionId, NftId), bool, ValueQuery>;

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
	pub type OriginOfShellsChosenParts<T: Config> =
		StorageMap<_, Twox64Concat, (CollectionId, NftId), ShellPartsOf<T>>;

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
			era: EraId,
		},
		/// Origin of Shell updated chosen parts.
		OriginOfShellChosenPartsUpdated {
			collection_id: CollectionId,
			nft_id: NftId,
			old_chosen_parts: Option<ShellPartsOf<T>>,
			new_chosen_parts: ShellPartsOf<T>,
		},
		/// Shell Collection ID is set.
		ShellCollectionIdSet { collection_id: CollectionId },
		/// Shell Parts Collection ID is set.
		ShellPartsCollectionIdSet { collection_id: CollectionId },
		/// Shell Part minted.
		ShellPartMinted {
			shell_parts_collection_id: CollectionId,
			shell_part_nft_id: NftId,
			parent_shell_collection_id: CollectionId,
			parent_shell_nft_id: NftId,
			owner: T::AccountId,
		},
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
		_Deprecated_MaxFoodFedLimitReached,
		CannotSetOriginOfShellChosenParts,
		AlreadySentFoodTwice,
		_Deprecated_NoFoodAvailable,
		NotOwner,
		NoPermission,
		WrongCollectionId,
		_Deprecated_NoHatchTimeDetected,
		ShellCollectionIdAlreadySet,
		ShellCollectionIdNotSet,
		RaceNotDetected,
		CareerNotDetected,
		RarityTypeNotDetected,
		GenerationNotDetected,
		FoodInfoUpdateError,
		ShellPartsCollectionIdAlreadySet,
		ShellPartsCollectionIdNotSet,
		ChosenPartsNotDetected,
		MissingShellPartMetadata,
		NoFoodLeftToFeedOriginOfShell,
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
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
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

		/// Sender tried to feed another Origin of Shell. This function will allocate a new daily
		/// daily ration of food if the sender has not sent food to another Origin of Shell within
		/// the current Era. If the sender has already sent food, the sender's FoodInfo will be
		/// mutated to update the food left and the origin of shells the sender has fed during the
		/// current Era.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic feeding the target Origin of Shell.
		/// - collection_id: The collection id of the Origin of Shell.
		/// - nft_id: The NFT id of the Origin of Shell.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
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
			let origin_of_shells_owned: u32 =
				pallet_uniques::pallet::Pallet::<T>::owned_in_collection(&collection_id.into(), &sender)
					.count() as u32;
			ensure!(origin_of_shells_owned > 0, Error::<T>::NoPermission);
			// Get Current Era
			let current_era = pallet_pw_nft_sale::Era::<T>::get();
			// Get number of times fed this era
			let num_of_times_fed =
				OriginOfShellFoodStats::<T>::get(current_era, (collection_id, nft_id));

			// Update account FoodInfo if not updated or create new FoodInfo for account's first
			// feeding
			FoodByOwners::<T>::try_mutate(&sender, |food_info| -> DispatchResult {
				let mut new_food_info = match food_info {
					None => Self::get_new_food_info(current_era, origin_of_shells_owned),
					Some(food_info) if current_era > food_info.era => {
						Self::get_new_food_info(current_era, origin_of_shells_owned)
					}
					Some(food_info) => food_info.clone(),
				};
				// Check if sender has food left to feed
				let food_left = new_food_info.food_left;
				ensure!(food_left > 0, Error::<T>::NoFoodLeftToFeedOriginOfShell);
				// Ensure sender hasn't fed the Origin of Shell 2 times
				let new_num_of_times_fed = match new_food_info
					.origin_of_shells_fed
					.get(&(collection_id.into(), nft_id.into()))
				{
					Some(num_times_fed) => {
						ensure!(
							*num_times_fed < T::MaxFoodFeedSelf::get(),
							Error::<T>::AlreadySentFoodTwice
						);
						let new_num_times_fed = *num_times_fed + 1u8;
						new_num_times_fed
					}
					None => 1u8,
				};
				new_food_info
					.origin_of_shells_fed
					.insert((collection_id.into(), nft_id.into()), new_num_of_times_fed);
				new_food_info.food_left = food_left - 1u32;
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
				era: current_era,
			});

			Ok(())
		}

		/// Hatch the origin_of_shell that is currently being hatched. This will trigger the end of
		/// the incubation process and the origin_of_shell will be burned. After burning, the user
		/// will receive the awakened Shell RMRK NFT and the nested NFT parts that renders the Shell
		/// NFT.
		///
		/// Parameters:
		/// - `origin`: Expected to be the `Overlord` account
		/// - `collection_id`: The collection id of the Origin of Shell RMRK NFT
		/// - `nft_id`: The NFT id of the Origin of Shell RMRK NFT
		/// - `default_shell_metadata`: File resource URI in decentralized storage for Shell NFT
		///	parts that render the Shell NFT
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn hatch_origin_of_shell(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			default_shell_metadata: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			// Ensure Overlord account is the sender
			let sender = ensure_signed(origin.clone())?;
			pallet_pw_nft_sale::pallet::Pallet::<T>::ensure_overlord(&sender)?;
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id.into()),
				Error::<T>::WrongCollectionId
			);
			let budget = budget::Value::new(T::NestingBudget::get());
			// Get owner of the Origin of Shell NFT
			let (owner, _) =
				pallet_rmrk_core::Pallet::<T>::lookup_root_owner(collection_id.into(), nft_id.into(), &budget)?;
			// Check if Shell Collection ID is set
			let shell_collection_id = Self::get_shell_collection_id()?;
			// Check if Shell Parts Collection ID is set
			let shell_parts_collection_id = Self::get_shell_parts_collection_id()?;
			// Get race, career, generation and rarity before burning origin of shell NFT
			let race =
				pallet_pw_nft_sale::Pallet::<T>::get_nft_property(collection_id.into(), nft_id.into(), "race")
					.ok_or(Error::<T>::RaceNotDetected)?;
			let career =
				pallet_pw_nft_sale::Pallet::<T>::get_nft_property(collection_id.into(), nft_id.into(), "career")
					.ok_or(Error::<T>::CareerNotDetected)?;
			let rarity_type_value =
				pallet_pw_nft_sale::Pallet::<T>::get_nft_property(collection_id.into(), nft_id.into(), "rarity")
					.ok_or(Error::<T>::RarityTypeNotDetected)?;
			let generation = pallet_pw_nft_sale::Pallet::<T>::get_nft_property(
				collection_id.into(),
				nft_id.into(),
				"generation",
			)
			.ok_or(Error::<T>::GenerationNotDetected)?;
			let race_type: RaceType =
				Decode::decode(&mut race.as_slice()).expect("[race] should not fail");
			let career_type: CareerType =
				Decode::decode(&mut career.as_slice()).expect("[career] should not fail");
			let rarity_type: RarityType = Decode::decode(&mut rarity_type_value.as_slice())
				.expect("[rarity] should not fail");
			let generation_id: GenerationId =
				Decode::decode(&mut generation.as_slice()).expect("[generation] should not fail");
			let parts_properties = vec![("generation", generation)];
			let mut shell_properties = parts_properties.clone();
			shell_properties.push(("race", race));
			shell_properties.push(("career", career));
			shell_properties.push(("rarity", rarity_type_value));

			// Get next expected Shell NFT ID
			let next_shell_nft_id =
				pallet_pw_nft_sale::Pallet::<T>::get_next_nft_id(shell_collection_id.into())?;
			// Burn Origin of Shell NFT then Mint Shell NFT
			pallet_rmrk_core::Pallet::<T>::burn_nft(
				Origin::<T>::Signed(owner.clone()).into(),
				collection_id.into(),
				nft_id.into(),
			)
			.map_err(|e| e.error)?;
			// Remove Properties from Uniques pallet
			pallet_pw_nft_sale::Pallet::<T>::remove_nft_properties(
				collection_id,
				nft_id,
				shell_properties.clone(),
			)?;
			let payee = pallet_pw_nft_sale::Pallet::<T>::payee()?;
			// Mint Shell NFT to Overlord to add properties and resource before sending to owner
			let (_, shell_nft_id) = pallet_rmrk_core::Pallet::<T>::nft_mint(
				owner.clone(),
				owner.clone(),
				next_shell_nft_id.into(),
				shell_collection_id.into(),
				Some(payee),
				Some(Permill::from_percent(1)),
				default_shell_metadata,
				true,
				None,
			)?;
			// Set Rarity Type, Race and Career properties for NFT
			pallet_pw_nft_sale::Pallet::<T>::set_nft_properties(
				shell_collection_id.into(),
				shell_nft_id,
				shell_properties,
			)?;

			// Iterate through the chosen parts
			let chosen_parts = OriginOfShellsChosenParts::<T>::take((collection_id, nft_id))
				.ok_or(Error::<T>::ChosenPartsNotDetected)?;
			for (slot_name, chosen_part) in chosen_parts.parts {
				let part_info = chosen_part.shell_part;
				let sub_parts = chosen_part.sub_parts;
				let metadata = match part_info.metadata {
					Some(metadata) => metadata,
					None => Default::default(),
				};
				// Add shell part properties
				let mut shell_part_properties = parts_properties.clone();
				let slot_name_value = property_value(&slot_name);
				// Check if part is tradeable
				let is_tradeable = part_info.tradeable;
				// Append shell part properties
				shell_part_properties.append(&mut vec![
					("name", property_value(&part_info.name)),
					("slot_name", slot_name_value.clone()),
					("rarity", property_value(&part_info.rarity)),
					("race", property_value(&part_info.race)),
					("career", property_value(&part_info.career)),
					("sizes", property_value(&part_info.sizes)),
					("style", property_value(&part_info.style)),
					("layer", property_value(&part_info.layer)),
					("x", property_value(&part_info.x)),
					("y", property_value(&part_info.y)),
				]);
				let (_, shell_part_nft_id) = Self::do_mint_shell_part_nft(
					owner.clone(),
					shell_part_properties,
					metadata,
					shell_parts_collection_id.into(),
					shell_collection_id.into(),
					shell_nft_id.into(),
					is_tradeable,
				)?;
				match sub_parts {
					Some(sub_parts) => {
						for sub_part_info in sub_parts {
							let sub_part_metadata = sub_part_info
								.metadata
								.ok_or(Error::<T>::MissingShellPartMetadata)?;
							// Add shell subpart properties
							let mut sub_part_properties = parts_properties.clone();
							// Append sub part properties
							sub_part_properties.append(&mut vec![
								("name", property_value(&sub_part_info.name)),
								("slot_name", slot_name_value.clone()),
								("rarity", property_value(&sub_part_info.rarity)),
								("race", property_value(&sub_part_info.race)),
								("career", property_value(&sub_part_info.career)),
								("sizes", property_value(&sub_part_info.sizes)),
								("style", property_value(&sub_part_info.style)),
								("layer", property_value(&sub_part_info.layer)),
								("x", property_value(&sub_part_info.x)),
								("y", property_value(&sub_part_info.y)),
							]);
							Self::do_mint_shell_part_nft(
								owner.clone(),
								sub_part_properties,
								sub_part_metadata,
								shell_parts_collection_id.into(),
								shell_parts_collection_id.into(),
								shell_part_nft_id,
								sub_part_info.tradeable,
							)?;
						}
					}
					None => (),
				}
			}

			Self::deposit_event(Event::ShellAwakened {
				shell_collection_id: shell_collection_id.into(),
				shell_nft_id: shell_nft_id.into(),
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
		#[pallet::call_index(3)]
		#[pallet::weight({0})]
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
		#[pallet::call_index(4)]
		#[pallet::weight({0})]
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
		#[pallet::call_index(5)]
		#[pallet::weight({0})]
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

		/// Privileged function to set the parts chosen by an account for a specific Origin of Shell.
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the chosen part to the Origin of Shell
		/// - `collection_id` - Collection ID of Origin of Shell
		/// - `nft_id` - NFT ID of the Origin of Shell
		/// - `chosen_parts` - Shell parts to be stored in Storage
		#[pallet::call_index(6)]
		#[pallet::weight({0})]
		pub fn set_origin_of_shell_chosen_parts(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			chosen_parts: ShellPartsOf<T>,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// Ensure that the collection is an Origin of Shell Collection
			ensure!(
				Self::is_origin_of_shell_collection_id(collection_id.into()),
				Error::<T>::WrongCollectionId
			);
			// Ensure the incubation process has started before setting chosen parts
			ensure!(
				HasOriginOfShellStartedIncubation::<T>::get((collection_id, nft_id)),
				Error::<T>::CannotSetOriginOfShellChosenParts
			);

			let old_chosen_parts = OriginOfShellsChosenParts::<T>::get((collection_id, nft_id));

			OriginOfShellsChosenParts::<T>::insert((collection_id, nft_id), chosen_parts.clone());

			Self::deposit_event(Event::OriginOfShellChosenPartsUpdated {
				collection_id,
				nft_id,
				old_chosen_parts,
				new_chosen_parts: chosen_parts,
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

	/// Helper function to get new FoodInfoOf<T> struct
	///
	/// Parameters:
	/// - `era`: The current Era of PhalaWorld to get new food info for the AccountId.
	/// - `multiply_food_by`: Number of owned Origin of Shell NFTs that will help calculate the
	/// product of `multiply_food_by` * `T::FoodPerEra`.
	fn get_new_food_info(era: EraId, multiply_food_by: u32) -> FoodInfo {
		let food_left = multiply_food_by * T::FoodPerEra::get();
		FoodInfo {
			era,
			origin_of_shells_fed: BTreeMap::new(),
			food_left,
		}
	}

	/// Helper function to mint a Top level Shell part. These are transferable Shell part NFTs.
	///
	/// Parameters:
	/// - `owner`: Root owner of the Shell part
	/// - `properties`: Vec of properties to be set
	/// - `metadata`: Metadata URI that will point to decentralized storage of the media
	/// file
	/// - `shell_parts_collection_id`: The Shell parts collection ID
	/// - `parent_collection_id`: Collection of the Shell NFT that owns the Shell Part
	/// - `parent_nft_id`: NFT ID of the Shell NFT that owns the Shell Part
	/// - `transferable`: If Part is transferable
	fn do_mint_shell_part_nft(
		owner: T::AccountId,
		properties: Vec<(&str, BoundedVec<u8, T::ValueLimit>)>,
		metadata: BoundedVec<u8, T::StringLimit>,
		shell_parts_collection_id: CollectionId,
		parent_collection_id: CollectionId,
		parent_nft_id: NftId,
		transferable: bool,
	) -> Result<(CollectionId, NftId), DispatchError> {
		// If metadata is None then metadata will be default
		// Get expected next available NFT ID to mint
		let next_nft_id =
			pallet_pw_nft_sale::Pallet::<T>::get_next_nft_id(shell_parts_collection_id.into())?;
		let payee = pallet_pw_nft_sale::Pallet::<T>::payee()?;
		// Mint Shell Part NFT directly to the Shell NFT
		let (_, shell_part_nft_id) = pallet_rmrk_core::Pallet::<T>::nft_mint_directly_to_nft(
			owner.clone(),
			(parent_collection_id.into(), parent_nft_id.into()),
			next_nft_id.into(),
			shell_parts_collection_id.into(),
			Some(payee),
			Some(Permill::from_percent(1)),
			metadata,
			transferable,
			None,
		)?;
		// Set Shell Part properties
		pallet_pw_nft_sale::Pallet::<T>::set_nft_properties(
			shell_parts_collection_id.into(),
			shell_part_nft_id,
			properties.clone(),
		)?;

		Self::deposit_event(Event::ShellPartMinted {
			shell_parts_collection_id,
			shell_part_nft_id: shell_part_nft_id.into(),
			parent_shell_collection_id: parent_collection_id,
			parent_shell_nft_id: parent_nft_id,
			owner,
		});

		Ok((shell_parts_collection_id, shell_part_nft_id.into()))
	}
}
