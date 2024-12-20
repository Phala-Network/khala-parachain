use alloc::collections::BTreeMap;
use codec::{alloc, Decode, Encode};
use frame_support::pallet_prelude::*;
use primitives::*;
use rmrk_traits::primitives::{CollectionId, NftId};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::cmp::Eq;

// Primitives
pub mod primitives {
	pub type PreorderId = u32;
	pub type EraId = u64;
	pub type GenerationId = u32;
}

/// Status types for different NFT Sale phases
#[derive(Encode, Decode, Debug, Clone, PartialEq, TypeInfo)]
pub enum StatusType {
	ClaimSpirits,
	PurchaseRareOriginOfShells,
	PurchasePrimeOriginOfShells,
	PreorderOriginOfShells,
	LastDayOfSale,
}

/// Purpose of an OverlordMessage
#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Purpose {
	RedeemSpirit,
	BuyPrimeOriginOfShells,
}

/// Overlord message with a purpose that will be signed by Overlord account
#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
pub struct OverlordMessage<AccountId> {
	pub account: AccountId,
	pub purpose: Purpose,
}

/// Rarity Types of Prime, Magic & Legendary
#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RarityType {
	Prime,
	Magic,
	Legendary,
}

/// Shell Part Rarity Types of Normal, Rare, Epic, Legend
#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PartRarityType {
	Normal,
	Rare,
	Epic,
	Legend,
}

/// Shell Part Sizes. For each race:
/// Cyborg have two sizes: MA, MB
/// X-Gene have three sizes: XA, XB, XC
/// Pandroid have four sizes: PA, PB, PC, PD
/// Ai Spectre only got one size: AA
#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PartSizeType {
	MA,
	MB,
	MC,
	FA,
	FB,
	FC,
	XA,
	XB,
	XC,
	PA,
	PB,
	PC,
	PD,
	AA,
}

/// Race types
#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RaceType {
	Cyborg,
	AISpectre,
	XGene,
	Pandroid,
}

/// Career types
#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CareerType {
	HackerWizard,
	HardwareDruid,
	RoboWarrior,
	TradeNegotiator,
	Web3Monk,
}

/// Preorder info for Non-Whitelist preorders
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PreorderInfo<AccountId, BoundedString> {
	/// Account owner of the Origin of Shell preorder
	pub owner: AccountId,
	/// Race type of the preorder
	pub race: RaceType,
	/// Career type of the preorder
	pub career: CareerType,
	/// Metadata of the owner
	pub metadata: BoundedString,
}

/// NFT sale types
#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum NftSaleType {
	ForSale,
	Giveaway,
	Reserved,
}

/// NftSaleInfo is used as the value in the StorageDoubleMap that takes key1 as the
/// RarityType and key2 as the RaceType
#[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct NftSaleInfo {
	/// Number of Race Type count
	pub race_count: u32,
	/// Number of races left to sell
	pub race_for_sale_count: u32,
	/// Number of giveaway races left
	pub race_giveaway_count: u32,
	/// Number of reserved races left
	pub race_reserved_count: u32,
}

/// Incubation Account Owner Food info
#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct FoodInfo {
	/// Era that an account last fed food to another Origin of Shell.
	pub era: EraId,
	/// A BoundedVec of (CollectionId, NftId).
	pub origin_of_shells_fed: BTreeMap<(CollectionId, NftId), u8>,
	/// Number of food left.
	pub food_left: u32,
}

/// Shell Parts info
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ShellPartInfo<BoundedString, BoundedSubParts> {
	pub shell_part: PartInfo<BoundedString>,
	/// If Metadata is None then this is a BoundedVec of ShellSubPartInfo that compose the Part
	pub sub_parts: Option<BoundedSubParts>,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PartInfo<BoundedString> {
	/// Name of the Part
	pub name: BoundedString,
	/// Shell part rarity type
	pub rarity: PartRarityType,
	/// Restriction for Shell Race, None is no restriction.
	pub race: Option<RaceType>,
	/// Restriction for Shell Career, None is no restriction.
	pub career: Option<CareerType>,
	/// Restriction for shell Sizes, None is no restriction.
	pub sizes: Option<BoundedVec<PartSizeType, ConstU32<4>>>,
	/// Style Code for Part
	pub style: Option<BoundedString>,
	/// Metadata is None if the Part is composed of Sub-Parts
	pub metadata: Option<BoundedString>,
	/// Layer in the png file
	pub layer: u32,
	/// x coordinate
	pub x: u32,
	/// y coordinate
	pub y: u32,
	/// Tradeable
	pub tradeable: bool,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ShellParts<BoundedString, BoundedParts> {
	pub parts: BTreeMap<BoundedString, BoundedParts>,
}

pub fn property_value<V: Encode, ValueLimit: Get<u32>>(value: &V) -> BoundedVec<u8, ValueLimit> {
	value
		.encode()
		.try_into()
		.expect("assume value can fit into property; qed.")
}
