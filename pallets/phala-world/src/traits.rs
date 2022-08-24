use alloc::collections::BTreeMap;
use codec::{alloc, Decode, Encode};
use frame_support::pallet_prelude::*;
use primitives::*;
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

/// Incubation Food info
#[derive(Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct FoodInfo<BoundedOriginOfShellsFed> {
	/// Era that an account last fed food to another Origin of Shell.
	pub era: EraId,
	/// A BoundedVec of (CollectionId, NftId)
	pub origin_of_shells_fed: BoundedOriginOfShellsFed,
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
	/// Shell shape
	pub shape: BoundedString,
	/// Is a special part
	pub special: bool,
	/// Metadata is None if the Part is composed of Sub-Parts
	pub metadata: Option<BoundedString>,
	/// Layer in the png file
	pub layer: u32,
	/// x coordinate
	pub x: u32,
	/// y coordinate
	pub y: u32,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ShellParts<BoundedString, BoundedParts> {
	pub parts: BTreeMap<BoundedString, BoundedParts>,
}
