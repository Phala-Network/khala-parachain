pub mod subbridge_chainid {
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	pub use sp_core::U256;
	use sp_runtime::RuntimeDebug;
	use sp_std::convert::{From, TryFrom};

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub enum SubBridgeChainId {
		Ethereum = 0,
		Khala = 1,
		Moonriver = 2,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub enum ChainBridgeChainId {
		Ethereum = 0,
		Khala = 1,
		Moonriver = 2,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub enum WanBridgeChainId {
		// Dummy for now
		Ethereum = 100,
		// Dummy for now
		Khala = 101,
	}

	impl TryFrom<SubBridgeChainId> for ChainBridgeChainId {
		type Error = ();

		fn try_from(chain_id: SubBridgeChainId) -> Result<Self, Self::Error> {
			match chain_id {
				SubBridgeChainId::Ethereum => Ok(ChainBridgeChainId::Ethereum),
				SubBridgeChainId::Khala => Ok(ChainBridgeChainId::Khala),
				SubBridgeChainId::Moonriver => Ok(ChainBridgeChainId::Moonriver),
			}
		}
	}

	impl TryFrom<SubBridgeChainId> for WanBridgeChainId {
		type Error = ();

		fn try_from(chain_id: SubBridgeChainId) -> Result<Self, Self::Error> {
			match chain_id {
				SubBridgeChainId::Ethereum => Ok(WanBridgeChainId::Ethereum),
				SubBridgeChainId::Khala => Ok(WanBridgeChainId::Khala),
				_ => Err(()),
			}
		}
	}

	impl From<ChainBridgeChainId> for SubBridgeChainId {
		fn from(chain_id: ChainBridgeChainId) -> Self {
			match chain_id {
				ChainBridgeChainId::Ethereum => SubBridgeChainId::Ethereum,
				ChainBridgeChainId::Khala => SubBridgeChainId::Khala,
				ChainBridgeChainId::Moonriver => SubBridgeChainId::Moonriver,
			}
		}
	}

	impl From<WanBridgeChainId> for SubBridgeChainId {
		fn from(chain_id: WanBridgeChainId) -> Self {
			match chain_id {
				WanBridgeChainId::Ethereum => SubBridgeChainId::Ethereum,
				WanBridgeChainId::Khala => SubBridgeChainId::Khala,
			}
		}
	}
	impl From<SubBridgeChainId> for u128 {
		fn from(chain_id: SubBridgeChainId) -> Self {
			chain_id as u128
		}
	}

	// ChainId is a type of u8 according to chainbridge implementation,
	// need an approach to covert from enum ChainBridgeChainId
	impl From<ChainBridgeChainId> for u8 {
		fn from(chain_id: ChainBridgeChainId) -> Self {
			chain_id as u8
		}
	}

	impl TryFrom<u8> for ChainBridgeChainId {
		type Error = ();

		fn try_from(chain_id: u8) -> Result<Self, Self::Error> {
			if chain_id == ChainBridgeChainId::Ethereum as u8 {
				Ok(ChainBridgeChainId::Ethereum)
			} else if chain_id == ChainBridgeChainId::Khala as u8 {
				Ok(ChainBridgeChainId::Khala)
			} else if chain_id == ChainBridgeChainId::Moonriver as u8 {
				Ok(ChainBridgeChainId::Moonriver)
			} else {
				Err(())
			}
		}
	}

	// ChainId is a type of U256 according to wanbridge implementation,
	// need an approach to covert from enum WanBridgeChainId
	impl From<WanBridgeChainId> for U256 {
		fn from(chain_id: WanBridgeChainId) -> Self {
			U256::from(chain_id as u8)
		}
	}

	impl TryFrom<U256> for WanBridgeChainId {
		type Error = ();

		fn try_from(chain_id: U256) -> Result<Self, Self::Error> {
			if chain_id == U256::from(WanBridgeChainId::Ethereum) {
				Ok(WanBridgeChainId::Ethereum)
			} else if chain_id == U256::from(WanBridgeChainId::Khala) {
				Ok(WanBridgeChainId::Khala)
			} else {
				Err(())
			}
		}
	}
}
