#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub mod migration;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::tokens::fungibles::{
			metadata::Mutate as FungibleMutate, Create as FungibleCerate,
			Transfer as FungibleTransfer,
		},
		traits::{Currency, ExistenceRequirement, StorageVersion},
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::traits::AccountIdConversion;
	use sp_std::{boxed::Box, cmp, convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};

	/// Const used to indicate chainbridge path. str "cb"
	pub const CB_ASSET_KEY: &[u8] = &[0x63, 0x62];
	/// const used to indicate celerbridge path. str "cr"
	pub const CR_PATH_KEY: &[u8] = &[0x63, 0x72];
	/// Account that would be reserved when register an asset
	pub const ASSETS_REGISTRY_ID: PalletId = PalletId(*b"phala/ar");

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub enum XBridgeConfig {
		Xcmp,
		ChainBridge {
			chain_id: u8,
			resource_id: [u8; 32],
			reserve_account: [u8; 32],
			is_mintable: bool,
		},
		// Potential other bridge solutions
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct XBridge {
		pub config: XBridgeConfig,
		pub metadata: Box<Vec<u8>>,
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct AssetProperties {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct AssetRegistryInfo {
		pub location: MultiLocation,
		pub reserve_location: Option<MultiLocation>,
		pub enabled_bridges: Vec<XBridge>,
		pub properties: AssetProperties,
		pub execution_price: Option<u128>,
	}

	pub trait GetAssetRegistryInfo<AssetId> {
		fn id(location: &MultiLocation) -> Option<AssetId>;
		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<MultiLocation>;
		fn decimals(id: &AssetId) -> Option<u8>;
		fn price(location: &MultiLocation) -> Option<(XcmAssetId, u128)>;
	}

	pub trait AccountId32Conversion {
		fn into_account(self) -> [u8; 32];
	}

	impl AccountId32Conversion for MultiLocation {
		fn into_account(self) -> [u8; 32] {
			sp_io::hashing::blake2_256(&self.encode())
		}
	}

	// Split `Reserve` location from the given MultiLocation.
	// The reserve location represent which chain the location belong to.
	// By finding the reserve location, we can also identity where an asset
	// comes from.
	pub trait ExtractReserveLocation {
		fn reserve_location(&self) -> Option<MultiLocation>;
	}

	impl ExtractReserveLocation for Junctions {
		fn reserve_location(&self) -> Option<MultiLocation> {
			match (self.at(0), self.at(1)) {
				(Some(GeneralKey(cb_key)), Some(GeneralIndex(chain_id)))
					if &cb_key == &CB_ASSET_KEY =>
				{
					Some(
						(
							0,
							X2(GeneralKey((&cb_key).to_vec()), GeneralIndex(*chain_id)),
						)
							.into(),
					)
				}
				_ => None,
			}
		}
	}

	impl ExtractReserveLocation for MultiLocation {
		fn reserve_location(&self) -> Option<MultiLocation> {
			match (self.parents, self.first_interior()) {
				// Sibling parachain
				(1, Some(Parachain(id))) => {
					let mut interior = self.interior.clone();
					// Remove Junction::Parachain
					interior.take_first();
					interior
						.reserve_location()
						.or(Some(MultiLocation::new(1, X1(Parachain(*id)))))
				}
				// Parent
				(1, _) => Some(MultiLocation::parent()),
				// Local
				(0, _) => self
					.interior
					.reserve_location()
					.or(Some(MultiLocation::here())),
				_ => None,
			}
		}
	}

	// Convert MultiLocation to a Chainbridge compatible resource id.
	pub trait IntoResourceId {
		fn into_rid(self, chain_id: u8) -> [u8; 32];
	}

	impl IntoResourceId for MultiLocation {
		fn into_rid(self, chain_id: u8) -> [u8; 32] {
			let mut rid = sp_io::hashing::blake2_256(&self.encode());
			rid[0] = chain_id;
			rid
		}
	}

	// Lookup asset location by its resource id.
	trait LookupByResourceId {
		fn lookup_by_rid(self, rid: [u8; 32]) -> Option<MultiLocation>;
	}

	pub trait NativeAssetChecker {
		fn is_native_asset(asset: &MultiAsset) -> bool;
		fn is_native_asset_location(id: &MultiLocation) -> bool;
		fn native_asset_location() -> MultiLocation;
	}

	pub struct NativeAssetFilter<T>(PhantomData<T>);
	impl<T: Get<ParaId>> NativeAssetChecker for NativeAssetFilter<T> {
		fn is_native_asset(asset: &MultiAsset) -> bool {
			match (&asset.id, &asset.fun) {
				// So far our native asset is concrete
				(Concrete(ref id), Fungible(_)) if Self::is_native_asset_location(id) => true,
				_ => false,
			}
		}

		fn is_native_asset_location(id: &MultiLocation) -> bool {
			let native_locations = [
				MultiLocation::here(),
				(1, X1(Parachain(T::get().into()))).into(),
			];
			native_locations.contains(id)
		}

		fn native_asset_location() -> MultiLocation {
			(1, X1(Parachain(T::get().into()))).into()
		}
	}

	// Should adapter the representation of asset location after reanchored.
	// Because xcm would reanchore the location of the asset that reserved on our chain.
	// https://github.com/paritytech/polkadot/pull/4470
	pub trait ReserveAssetChecker {
		// Return true if asset is reserved on local
		fn is_asset_reserved_locally(asset: &MultiAsset) -> bool;
		// Return true if given location is reserved on local
		fn is_location_reserved_locally(id: &MultiLocation) -> bool;
		// Return location reprented whithin gloable consensus if asset is reserved on local, otherwise return None
		fn to_globalconsensus_location(location: &MultiLocation) -> Option<MultiLocation>;
		// Return a new asset with a global consensusus location if asset is reserved on local.
		fn to_gloableconsensus_asset(asset: &MultiAsset) -> MultiAsset;
	}

	pub struct ReserveAssetFilter<T, I>(PhantomData<(T, I)>);
	impl<T: Get<ParaId>, I: NativeAssetChecker> ReserveAssetChecker for ReserveAssetFilter<T, I> {
		fn is_asset_reserved_locally(asset: &MultiAsset) -> bool {
			match &asset.id {
				Concrete(ref id) if Self::is_location_reserved_locally(id) => true,
				_ => false,
			}
		}

		fn is_location_reserved_locally(id: &MultiLocation) -> bool {
			if let Some(location) = Self::to_globalconsensus_location(id) {
				match (location.parents, location.first_interior()) {
					(1, Some(para_id)) => *para_id == Parachain(T::get().into()),
					_ => false,
				}
			} else {
				false
			}
		}

		fn to_globalconsensus_location(location: &MultiLocation) -> Option<MultiLocation> {
			match (location.parents, location.first_interior()) {
				// We should handle (0, Here) specially case we can not push interior front directly
				(0, None) => Some((1, Parachain(T::get().into())).into()),
				(0, Some(_)) => {
					let mut origin_location = location.clone();
					origin_location.parents = 1;
					return match origin_location
						.interior
						.push_front(Parachain(T::get().into()))
					{
						Ok(()) => Some(origin_location),
						Err(_) => None,
					};
				}
				_ => Some(location.clone()),
			}
		}

		fn to_gloableconsensus_asset(asset: &MultiAsset) -> MultiAsset {
			match &asset.id {
				Concrete(ref id) if Self::is_location_reserved_locally(id) => (
					Concrete(
						Self::to_globalconsensus_location(id)
							.unwrap_or(id.clone())
							.into(),
					),
					asset.fun.clone(),
				)
					.into(),
				// Asset location already reprensted by gloable consensus if it is non-reserved asset for us.
				_ => asset.clone(),
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type RegistryCommitteeOrigin: EnsureOrigin<Self::Origin>;
		type Currency: Currency<Self::AccountId>;
		#[pallet::constant]
		type MinBalance: Get<<Self as pallet_assets::Config>::Balance>;
		#[pallet::constant]
		type NativeExecutionPrice: Get<u128>;
		type NativeAssetChecker: NativeAssetChecker;
		type ReserveAssetChecker: ReserveAssetChecker;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	const LOG_TARGET: &str = "runtime::asset-registry";

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Mapping fungible asset location to corresponding asset id
	#[pallet::storage]
	#[pallet::getter(fn location_to_id)]
	pub type IdByLocations<T: Config> =
		StorageMap<_, Twox64Concat, MultiLocation, <T as pallet_assets::Config>::AssetId>;

	/// Mapping fungible asset resource id to corresponding asset id
	#[pallet::storage]
	#[pallet::getter(fn rid_to_id)]
	pub type IdByResourceId<T: Config> =
		StorageMap<_, Twox64Concat, [u8; 32], <T as pallet_assets::Config>::AssetId>;

	// Mapping fungible assets id to corresponding registry info
	#[pallet::storage]
	#[pallet::getter(fn id_to_registry_info)]
	pub type RegistryInfoByIds<T: Config> =
		StorageMap<_, Twox64Concat, <T as pallet_assets::Config>::AssetId, AssetRegistryInfo>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset is registerd.
		AssetRegistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			location: MultiLocation,
		},
		/// Asset is unregisterd.
		AssetUnregistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			location: MultiLocation,
		},
		/// Asset enabled chainbridge.
		ChainbridgeEnabled {
			asset_id: <T as pallet_assets::Config>::AssetId,
			chain_id: u8,
			resource_id: [u8; 32],
		},
		/// Asset disabled chainbridge.
		ChainbridgeDisabled {
			asset_id: <T as pallet_assets::Config>::AssetId,
			chain_id: u8,
			resource_id: [u8; 32],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetAlreadyExist,
		AssetNotRegistered,
		BridgeAlreadyEnabled,
		BridgeAlreadyDisabled,
		FailedToTransactAsset,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_assets::Config,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128>,
	{
		/// Force withdraw some amount of assets from ASSETS_REGISTRY_ID, if the given asset_id is None,
		/// would performance withdraw PHA from this account
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_withdraw_fund(
			origin: OriginFor<T>,
			asset_id: Option<T::AssetId>,
			recipient: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin)?;
			let fund_account = ASSETS_REGISTRY_ID.into_account_truncating();
			if let Some(asset_id) = asset_id {
				<pallet_assets::pallet::Pallet<T> as FungibleTransfer<T::AccountId>>::transfer(
					asset_id,
					&fund_account,
					&recipient,
					amount.into(),
					true,
				)
				.map_err(|_| Error::<T>::FailedToTransactAsset)?;
			} else {
				<T as Config>::Currency::transfer(
					&fund_account,
					&recipient,
					amount.into(),
					ExistenceRequirement::AllowDeath,
				)?;
			}
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_register_asset(
			origin: OriginFor<T>,
			location: MultiLocation,
			asset_id: T::AssetId,
			properties: AssetProperties,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin.clone())?;
			// Ensure location has not been registered
			ensure!(
				IdByLocations::<T>::get(&location) == None,
				Error::<T>::AssetAlreadyExist
			);
			// Ensure asset_id has not been registered
			ensure!(
				RegistryInfoByIds::<T>::get(&asset_id) == None,
				Error::<T>::AssetAlreadyExist
			);
			// Set bridge account as asset's owner/issuer/admin/freezer
			<pallet_assets::pallet::Pallet<T> as FungibleCerate<T::AccountId>>::create(
				asset_id,
				ASSETS_REGISTRY_ID.into_account_truncating(),
				true,
				Self::default_asset_ed(properties.decimals),
			)?;
			IdByLocations::<T>::insert(&location, asset_id);
			RegistryInfoByIds::<T>::insert(
				asset_id,
				AssetRegistryInfo {
					location: location.clone(),
					reserve_location: location.clone().reserve_location(),
					// Xcmp will be enabled when assets being registered.
					enabled_bridges: vec![XBridge {
						config: XBridgeConfig::Xcmp,
						metadata: Box::new(Vec::new()),
					}],
					properties: properties.clone(),
					execution_price: None,
				},
			);
			<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::set(
				asset_id,
				&ASSETS_REGISTRY_ID.into_account_truncating(),
				properties.name,
				properties.symbol,
				properties.decimals,
			)?;

			Self::deposit_event(Event::AssetRegistered { asset_id, location });
			Ok(())
		}

		/// Clean asset info stored in asset wrapper, not call pallet_assets::destory(),
		/// By cleaning them in current pallet, xcm and bridge transfering on this asset
		/// will not success anymore, we should call pallet_assets::destory() manually
		/// if we want to delete this asset from our chain
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_unregister_asset(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin)?;
			let info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;

			IdByLocations::<T>::remove(&info.location);

			// Unbind resource id and asset id if have chain bridge set
			for bridge in info.enabled_bridges {
				if let XBridgeConfig::ChainBridge {
					chain_id,
					resource_id,
					..
				} = bridge.config
				{
					log::trace!(
						target: LOG_TARGET,
						"Found enabled chainbridge, chain_id ${:?}.",
						chain_id,
					);
					IdByResourceId::<T>::remove(&resource_id);
				}
			}
			// Delete registry info
			RegistryInfoByIds::<T>::remove(&asset_id);

			Self::deposit_event(Event::AssetUnregistered {
				asset_id,
				location: info.location,
			});
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_set_metadata(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			properties: AssetProperties,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin.clone())?;

			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			info.properties = properties.clone();
			RegistryInfoByIds::<T>::insert(&asset_id, &info);
			<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::set(
				asset_id,
				&ASSETS_REGISTRY_ID.into_account_truncating(),
				properties.name,
				properties.symbol,
				properties.decimals,
			)?;

			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_set_price(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			execution_price: u128,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin.clone())?;

			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			info.execution_price = Some(execution_price);
			RegistryInfoByIds::<T>::insert(&asset_id, &info);
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_enable_chainbridge(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			chain_id: u8,
			is_mintable: bool,
			metadata: Box<Vec<u8>>,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin)?;
			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let resource_id: [u8; 32] = info.location.clone().into_rid(chain_id);

			ensure!(
				IdByResourceId::<T>::get(&resource_id) == None,
				Error::<T>::BridgeAlreadyEnabled,
			);
			IdByResourceId::<T>::insert(&resource_id, &asset_id);
			// Save into registry info, here save chain id can not be added more than twice
			let reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(chain_id as u128),
				),
			)
				.into();
			info.enabled_bridges.push(XBridge {
				config: XBridgeConfig::ChainBridge {
					chain_id,
					resource_id: resource_id.clone(),
					reserve_account: reserve_location.into_account(),
					is_mintable,
				},
				metadata,
			});
			RegistryInfoByIds::<T>::insert(&asset_id, &info);

			Self::deposit_event(Event::ChainbridgeEnabled {
				asset_id,
				chain_id,
				resource_id,
			});
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_disable_chainbridge(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			chain_id: u8,
		) -> DispatchResult {
			T::RegistryCommitteeOrigin::ensure_origin(origin)?;
			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let resource_id: [u8; 32] = info.location.clone().into_rid(chain_id);

			ensure!(
				IdByResourceId::<T>::get(&resource_id).is_some(),
				Error::<T>::BridgeAlreadyDisabled,
			);
			// Unbind resource id and asset id
			IdByResourceId::<T>::remove(&resource_id);
			// Remove chainbridge info
			if let Some(idx) = info
				.enabled_bridges
				.iter()
				.position(|item| match item.config {
					XBridgeConfig::ChainBridge {
						chain_id: cid,
						resource_id: rid,
						..
					} => cid == chain_id && rid == resource_id,
					_ => false,
				}) {
				info.enabled_bridges.remove(idx);
			}
			RegistryInfoByIds::<T>::insert(&asset_id, &info);

			Self::deposit_event(Event::ChainbridgeDisabled {
				asset_id,
				chain_id,
				resource_id,
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn default_price(native_price: u128, decimals: u8) -> u128 {
			if decimals >= 12 {
				native_price.saturating_mul(10u128.saturating_pow(decimals as u32 - 12))
			} else {
				native_price.saturating_div(10u128.saturating_pow(12 - decimals as u32))
			}
		}

		fn default_asset_ed(decimals: u8) -> <T as pallet_assets::Config>::Balance {
			let native_ed: u128 = T::MinBalance::get().into();
			if decimals >= 12 {
				native_ed
					.saturating_mul(10u128.saturating_pow(decimals as u32 - 12))
					.into()
			} else {
				// + 1 make sure min balance always > 0
				cmp::max(
					native_ed.saturating_div(10u128.saturating_pow(12 - decimals as u32)),
					1,
				)
				.into()
			}
		}

		fn convert_location_to_id(
			location: &MultiLocation,
		) -> Option<<T as pallet_assets::Config>::AssetId> {
			IdByLocations::<T>::get(location).or_else(|| {
				if let Some(globalconsensus_location) =
					T::ReserveAssetChecker::to_globalconsensus_location(location)
				{
					IdByLocations::<T>::get(globalconsensus_location)
				} else {
					None
				}
			})
		}
	}

	impl<T: Config> GetAssetRegistryInfo<<T as pallet_assets::Config>::AssetId> for Pallet<T>
	where
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn id(asset: &MultiLocation) -> Option<<T as pallet_assets::Config>::AssetId> {
			Self::convert_location_to_id(asset)
		}

		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<MultiLocation> {
			IdByResourceId::<T>::get(resource_id)
				.and_then(|id| RegistryInfoByIds::<T>::get(&id).map(|m| m.location))
		}

		fn decimals(id: &<T as pallet_assets::Config>::AssetId) -> Option<u8> {
			RegistryInfoByIds::<T>::get(&id).map(|m| m.properties.decimals)
		}

		fn price(location: &MultiLocation) -> Option<(XcmAssetId, u128)> {
			// We should handle native asset specially because we never register it
			if T::NativeAssetChecker::is_native_asset_location(location) {
				return Some((location.clone().into(), T::NativeExecutionPrice::get()));
			}

			Self::convert_location_to_id(location).and_then(|id| {
				RegistryInfoByIds::<T>::get(&id).map(|m| {
					(
						// Here we must return location passed by parameter in case it's the local consensus location of asset
						location.clone().into(),
						// If the registered asset has not set a price, return default price according to native asset price and its decimals
						m.execution_price.unwrap_or(Self::default_price(
							T::NativeExecutionPrice::get(),
							m.properties.decimals,
						)),
					)
				})
			})
		}
	}

	#[cfg(test)]
	mod tests {
		use crate as assets_registry;
		use assets_registry::{
			mock::*, AccountId32Conversion, AssetProperties, GetAssetRegistryInfo,
			ASSETS_REGISTRY_ID,
		};
		use frame_support::{assert_noop, assert_ok};
		use sp_runtime::{traits::AccountIdConversion, AccountId32, DispatchError};

		#[test]
		fn test_withdraw_fund_of_pha() {
			let recipient: AccountId32 =
				MultiLocation::new(0, X1(GeneralKey(b"recipient".to_vec())))
					.into_account()
					.into();
			new_test_ext().execute_with(|| {
				assert_eq!(
					Balances::free_balance(&ASSETS_REGISTRY_ID.into_account_truncating()),
					ENDOWED_BALANCE
				);
				assert_ok!(AssetsRegistry::force_withdraw_fund(
					Origin::root(),
					None,
					recipient.clone(),
					10,
				));
				assert_eq!(
					Balances::free_balance(&ASSETS_REGISTRY_ID.into_account_truncating()),
					ENDOWED_BALANCE - 10
				);
				assert_eq!(Balances::free_balance(&recipient), 10);
			});
		}

		#[test]
		fn test_withdraw_fund_of_asset() {
			let recipient: AccountId32 =
				MultiLocation::new(0, X1(GeneralKey(b"recipient".to_vec())))
					.into_account()
					.into();
			let fund_account: <Test as frame_system::Config>::AccountId =
				ASSETS_REGISTRY_ID.into_account_truncating();

			new_test_ext().execute_with(|| {
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					MultiLocation::new(1, Here).into(),
					0,
					assets_registry::AssetProperties {
						name: b"Kusama".to_vec(),
						symbol: b"KSM".to_vec(),
						decimals: 12,
					},
				));

				// Only ASSETS_REGISTRY_ID has mint permission
				assert_ok!(Assets::mint(
					Origin::signed(fund_account.clone()),
					0,
					fund_account.clone().into(),
					1_000
				));
				assert_eq!(Assets::balance(0u32.into(), &fund_account), 1_000);

				assert_ok!(AssetsRegistry::force_withdraw_fund(
					Origin::root(),
					Some(0),
					recipient.clone(),
					10,
				));
				assert_eq!(Assets::balance(0u32.into(), &fund_account), 1_000 - 10);
				assert_eq!(Assets::balance(0u32.into(), &recipient), 10);
			});
		}

		#[test]
		fn test_asset_register() {
			new_test_ext().execute_with(|| {
				// Register first asset, id = 0
				let para_a_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(1)),
				};

				// Should be failed if origin is not sudo user
				assert_noop!(
					AssetsRegistry::force_register_asset(
						Some(ALICE).into(),
						para_a_location.clone().into(),
						0,
						AssetProperties {
							name: b"ParaAAsset".to_vec(),
							symbol: b"PAA".to_vec(),
							decimals: 12,
						},
					),
					DispatchError::BadOrigin
				);

				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_a_location.clone().into(),
					0,
					AssetProperties {
						name: b"ParaAAsset".to_vec(),
						symbol: b"PAA".to_vec(),
						decimals: 12,
					},
				));

				assert_events(vec![Event::AssetsRegistry(
					assets_registry::Event::AssetRegistered {
						asset_id: 0u32.into(),
						location: para_a_location.clone(),
					},
				)]);
				assert_eq!(AssetsRegistry::id(&para_a_location).unwrap(), 0u32);
				assert_eq!(Assets::total_supply(0u32.into()), 0);
				assert_eq!(AssetsRegistry::decimals(&0u32.into()).unwrap(), 12u8);

				// Force set metadata
				assert_ok!(AssetsRegistry::force_set_metadata(
					Origin::root(),
					0,
					AssetProperties {
						name: b"ParaAAAAsset".to_vec(),
						symbol: b"PAAAA".to_vec(),
						decimals: 18,
					},
				));
				assert_eq!(AssetsRegistry::decimals(&0u32.into()).unwrap(), 18u8);

				// Same asset location register again, should be failed
				assert_noop!(
					AssetsRegistry::force_register_asset(
						Origin::root(),
						para_a_location.clone().into(),
						1,
						AssetProperties {
							name: b"ParaAAsset".to_vec(),
							symbol: b"PAA".to_vec(),
							decimals: 12,
						},
					),
					assets_registry::Error::<Test>::AssetAlreadyExist
				);

				let para_b_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(2)),
				};

				// Same asset id register again, should be failed
				assert_noop!(
					AssetsRegistry::force_register_asset(
						Origin::root(),
						para_b_location.clone().into(),
						0,
						AssetProperties {
							name: b"ParaBAsset".to_vec(),
							symbol: b"PBA".to_vec(),
							decimals: 12,
						},
					),
					assets_registry::Error::<Test>::AssetAlreadyExist
				);

				// Register another asset, id = 1
				let para_b_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(2)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_b_location.clone().into(),
					1,
					AssetProperties {
						name: b"ParaBAsset".to_vec(),
						symbol: b"PBA".to_vec(),
						decimals: 12,
					},
				));
				assert_eq!(AssetsRegistry::id(&para_b_location).unwrap(), 1u32);

				// Unregister asset
				assert_ok!(AssetsRegistry::force_unregister_asset(Origin::root(), 1));
				assert_eq!(AssetsRegistry::id(&para_b_location), None);
			});
		}

		#[test]
		fn test_non_registered_asset_price() {
			new_test_ext().execute_with(|| {
				let para_a_location = MultiLocation {
					parents: 1,
					interior: X1(Parachain(1)),
				};

				assert_eq!(AssetsRegistry::price(&para_a_location), None);
			})
		}

		#[test]
		fn test_registered_asset_with_default_price() {
			new_test_ext().execute_with(|| {
				let para_a_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(1)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_a_location.clone().into(),
					0,
					AssetProperties {
						name: b"ParaAAsset".to_vec(),
						symbol: b"PAA".to_vec(),
						decimals: 12,
					},
				));
				assert_eq!(
					AssetsRegistry::price(&para_a_location),
					Some((para_a_location.into(), NativeExecutionPrice::get()))
				);

				let para_b_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(2)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_b_location.clone().into(),
					1,
					AssetProperties {
						name: b"ParaBAsset".to_vec(),
						symbol: b"PBA".to_vec(),
						decimals: 18,
					},
				));
				assert_eq!(
					AssetsRegistry::price(&para_b_location),
					Some((
						para_b_location.into(),
						NativeExecutionPrice::get().saturating_mul(10u128.saturating_pow(6))
					))
				);

				let para_c_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(3)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_c_location.clone().into(),
					2,
					AssetProperties {
						name: b"ParaCAsset".to_vec(),
						symbol: b"PCA".to_vec(),
						decimals: 6,
					},
				));
				assert_eq!(
					AssetsRegistry::price(&para_c_location),
					Some((
						para_c_location.into(),
						NativeExecutionPrice::get().saturating_div(10u128.saturating_pow(6))
					))
				);
			})
		}

		#[test]
		fn test_registered_asset_with_specific_price() {
			new_test_ext().execute_with(|| {
				let para_a_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(1)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_a_location.clone().into(),
					0,
					AssetProperties {
						name: b"ParaAAsset".to_vec(),
						symbol: b"PAA".to_vec(),
						decimals: 12,
					},
				));
				assert_ok!(AssetsRegistry::force_set_price(
					Origin::root(),
					0,
					NativeExecutionPrice::get() * 2,
				));
				assert_eq!(
					AssetsRegistry::price(&para_a_location),
					Some((
						para_a_location.clone().into(),
						NativeExecutionPrice::get().saturating_mul(2)
					))
				);
				assert_ok!(AssetsRegistry::force_set_price(
					Origin::root(),
					0,
					NativeExecutionPrice::get() * 4,
				));
				assert_eq!(
					AssetsRegistry::price(&para_a_location),
					Some((
						para_a_location.into(),
						NativeExecutionPrice::get().saturating_mul(4)
					))
				);

				let para_b_location: MultiLocation = MultiLocation {
					parents: 1,
					interior: X1(Parachain(2)),
				};
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					para_b_location.clone().into(),
					1,
					AssetProperties {
						name: b"ParaBAsset".to_vec(),
						symbol: b"PBA".to_vec(),
						decimals: 18,
					},
				));
				assert_ok!(AssetsRegistry::force_set_price(
					Origin::root(),
					1,
					NativeExecutionPrice::get() * 2,
				));
				assert_eq!(
					AssetsRegistry::price(&para_b_location),
					Some((
						para_b_location.clone().into(),
						NativeExecutionPrice::get().saturating_mul(2)
					))
				);
				assert_ok!(AssetsRegistry::force_set_price(
					Origin::root(),
					1,
					NativeExecutionPrice::get() * 2u128.saturating_mul(10u128.saturating_pow(6)),
				));
				assert_eq!(
					AssetsRegistry::price(&para_b_location),
					Some((
						para_b_location.into(),
						NativeExecutionPrice::get()
							* 2u128.saturating_mul(10u128.saturating_pow(6))
					))
				);
			})
		}
	}
}
