// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{
    AccountId, ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent,
    RuntimeOrigin,
};
use frame_support::{
    match_types, parameter_types,
    traits::{ConstU32, Everything, Nothing},
};
use frame_system::EnsureRoot;
use xcm::latest::{prelude::*, Weight as XCMWeight};
use xcm_builder::{
    AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds,
    ParentIsPreset, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::traits::WithOriginFilter;

parameter_types! {
    pub const RelayNetwork: NetworkId = NetworkId::Kusama;
    pub UniversalLocation: InteriorMultiLocation =
        X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
}

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<ParentIsPreset<AccountId>, RuntimeOrigin>,
);

match_types! {
    pub type JustTheParent: impl Contains<MultiLocation> = { MultiLocation { parents:1, interior: Here } };
}
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

parameter_types! {
    pub UnitWeightCost: XCMWeight = XCMWeight::from_parts(200_000_000u64, 0);
    pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = (); // balances not supported
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = (); // balances not supported
    type IsTeleporter = (); // balances not supported
    type UniversalLocation = UniversalLocation;
    type Barrier = AllowUnpaidExecutionFrom<JustTheParent>;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>; // balances not supported
    type Trader = (); // balances not supported
    type ResponseHandler = (); // Don't handle responses for now.
    type AssetTrap = (); // don't trap for now
    type AssetClaims = (); // don't claim for now
    type SubscriptionService = (); // don't handle subscriptions for now
    type PalletInstancesInfo = crate::AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<Everything>;
    type SafeCallFilter = Everything;
	type Aliasers = Nothing;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}
impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// No local origins on this chain are allowed to dispatch XCM sends.
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = crate::Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = ();
    type MaxLockers = ConstU32<8>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
    type AdminOrigin = EnsureRoot<AccountId>;
}
