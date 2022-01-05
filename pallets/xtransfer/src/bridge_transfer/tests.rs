#![cfg(test)]

use crate::assets_wrapper::pallet::{AccountId32Conversion, XTransferAssetInfo, CB_ASSET_KEY};
use crate::bridge;
use crate::bridge_transfer::mock::{
	assert_events, balances, expect_event, new_test_ext, Assets, AssetsWrapper, Balances, Bridge,
	BridgeTransfer, Call, Event, NativeTokenResourceId, Origin, ProposalLifetime, Test, ALICE,
	ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;
use xcm::latest::prelude::*;

const TEST_THRESHOLD: u32 = 2;

fn make_transfer_proposal(dest: Vec<u8>, amount: u64) -> Call {
	let resource_id = NativeTokenResourceId::get();
	Call::BridgeTransfer(crate::bridge_transfer::Call::transfer {
		dest,
		amount: amount.into(),
		rid: resource_id,
	})
}

#[test]
fn register_asset() {
	new_test_ext().execute_with(|| {
		let bridge_asset_location = MultiLocation::new(
			1,
			X4(
				Parachain(2004),
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(0),
				GeneralKey(b"an asset".to_vec()),
			),
		);

		// permission denied
		assert_noop!(
			AssetsWrapper::force_register_asset(
				Origin::signed(ALICE),
				bridge_asset_location.clone().into(),
				0,
				ALICE,
			),
			DispatchError::BadOrigin
		);

		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
		));

		// same location register again, should be failed
		assert_noop!(
			AssetsWrapper::force_register_asset(
				Origin::root(),
				bridge_asset_location.clone().into(),
				1,
				ALICE,
			),
			crate::pallet_assets_wrapper::Error::<Test>::AssetAlreadyExist
		);

		let another_bridge_asset_location = MultiLocation::new(
			1,
			X4(
				Parachain(2004),
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(0),
				GeneralKey(b"another asset".to_vec()),
			),
		);

		// same asset id register again, should be failed
		assert_noop!(
			AssetsWrapper::force_register_asset(
				Origin::root(),
				another_bridge_asset_location.clone().into(),
				0,
				ALICE,
			),
			crate::pallet_assets_wrapper::Error::<Test>::AssetAlreadyExist
		);

		// register another asset, id = 1
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			another_bridge_asset_location.clone().into(),
			1,
			ALICE,
		));
		assert_eq!(
			AssetsWrapper::id(&another_bridge_asset_location.clone().into()).unwrap(),
			1u32
		);
		assert_eq!(
			AssetsWrapper::asset(&1u32.into()).unwrap(),
			another_bridge_asset_location.clone().into()
		);

		// unregister asset
		assert_ok!(AssetsWrapper::force_unregister_asset(Origin::root(), 1));
		assert_eq!(
			AssetsWrapper::id(&another_bridge_asset_location.into()),
			None
		);
		assert_eq!(AssetsWrapper::asset(&1u32.into()), None);
	})
}

#[test]
fn transfer_assets_not_registered() {
	new_test_ext().execute_with(|| {
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
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain));
		assert_ok!(BridgeTransfer::update_fee(Origin::root(), 2, 2, dest_chain));

		assert_noop!(
			BridgeTransfer::transfer_assets(
				Origin::signed(ALICE),
				bridge_asset_location.into(),
				dest_chain,
				recipient.clone(),
				amount,
			),
			crate::bridge_transfer::Error::<Test>::AssetNotRegistered
		);
	})
}

#[test]
fn transfer_assets_insufficient_balance() {
	new_test_ext().execute_with(|| {
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
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain));
		assert_ok!(BridgeTransfer::update_fee(Origin::root(), 2, 2, dest_chain));

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
		));

		// setup solo chain for this asset
		assert_ok!(AssetsWrapper::force_setup_solochain(
			Origin::root(),
			0,
			dest_chain,
		));

		// after registered, free balance of ALICE is 0
		assert_noop!(
			BridgeTransfer::transfer_assets(
				Origin::signed(ALICE),
				bridge_asset_location.into(),
				dest_chain,
				recipient.clone(),
				amount,
			),
			crate::bridge_transfer::Error::<Test>::InsufficientBalance
		);
	})
}

#[test]
fn transfer_assets_to_nonreserve() {
	new_test_ext().execute_with(|| {
		let dest_chain: u8 = 2;
		let dest_reserve_location: MultiLocation = (
			0,
			X2(
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(dest_chain.into()),
			),
		)
			.into();
		let bridge_asset_location = MultiLocation::new(
			1,
			X4(
				Parachain(2004),
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(0),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain));
		assert_ok!(BridgeTransfer::update_fee(Origin::root(), 2, 2, dest_chain));

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
		));

		// setup solo chain for this asset
		assert_ok!(AssetsWrapper::force_setup_solochain(
			Origin::root(),
			0,
			dest_chain,
		));

		// mint some token to ALICE
		assert_ok!(Assets::mint(Origin::signed(ALICE), 0, ALICE, amount * 2));
		assert_eq!(Assets::balance(0, &ALICE), amount * 2);

		assert_ok!(BridgeTransfer::transfer_assets(
			Origin::signed(ALICE),
			bridge_asset_location.into(),
			dest_chain,
			recipient.clone(),
			amount,
		));

		assert_eq!(Assets::balance(0, &ALICE), amount);
		// the asset's reserve chain is 0, dest chain is 2,
		// so will save asset into reserve account of dest chain
		assert_eq!(
			Assets::balance(0, &dest_reserve_location.into_account().into()),
			amount
		);
	})
}

#[test]
fn transfer_assets_to_reserve() {
	new_test_ext().execute_with(|| {
		let dest_chain: u8 = 2;
		let dest_reserve_location: MultiLocation = (
			0,
			X2(
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(dest_chain.into()),
			),
		)
			.into();
		let bridge_asset_location = MultiLocation::new(
			1,
			X4(
				Parachain(2004),
				GeneralKey(CB_ASSET_KEY.to_vec()),
				GeneralIndex(dest_chain.into()),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain));
		assert_ok!(BridgeTransfer::update_fee(Origin::root(), 2, 2, dest_chain));

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
		));

		// setup solo chain for this asset
		assert_ok!(AssetsWrapper::force_setup_solochain(
			Origin::root(),
			0,
			dest_chain,
		));

		// mint some token to ALICE
		assert_ok!(Assets::mint(Origin::signed(ALICE), 0, ALICE, amount * 2));
		assert_eq!(Assets::balance(0, &ALICE), amount * 2);

		assert_ok!(BridgeTransfer::transfer_assets(
			Origin::signed(ALICE),
			bridge_asset_location.into(),
			dest_chain,
			recipient.clone(),
			amount,
		));

		assert_eq!(Assets::balance(0, &ALICE), amount);
		// the asset's reserve chain is 2, dest chain is 2,
		// so assets just be burned from sender
		assert_eq!(
			Assets::balance(0, &dest_reserve_location.into_account().into()),
			0
		);
	})
}

#[test]
fn transfer_native() {
	new_test_ext().execute_with(|| {
		let dest_chain = 0;
		let resource_id = NativeTokenResourceId::get();
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain));
		assert_ok!(BridgeTransfer::update_fee(Origin::root(), 2, 2, dest_chain));

		assert_noop!(
			BridgeTransfer::transfer_native(
				Origin::signed(RELAYER_A),
				Balances::free_balance(RELAYER_A),
				recipient.clone(),
				dest_chain,
			),
			crate::bridge_transfer::Error::<Test>::InsufficientBalance
		);

		assert_ok!(BridgeTransfer::transfer_native(
			Origin::signed(RELAYER_A),
			amount.clone(),
			recipient.clone(),
			dest_chain,
		));

		expect_event(bridge::Event::FungibleTransfer(
			dest_chain,
			1,
			resource_id,
			amount.into(),
			recipient,
		));

		assert_eq!(
			Balances::free_balance(&Bridge::account_id()),
			ENDOWED_BALANCE + amount
		)
	})
}

#[test]
fn simulate_transfer_pha_from_solochain() {
	new_test_ext().execute_with(|| {
		// Check inital state
		let bridge_account = Bridge::account_id();
		let resource_id = NativeTokenResourceId::get();
		assert_eq!(Balances::free_balance(&bridge_account), ENDOWED_BALANCE);
		let relayer_location = MultiLocation::new(
			0,
			X1(AccountId32 {
				network: NetworkId::Any,
				id: RELAYER_A.into(),
			}),
		);

		// Transfer and check result
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			relayer_location.encode(),
			10,
			resource_id,
		));
		assert_eq!(
			Balances::free_balance(&bridge_account),
			ENDOWED_BALANCE - 10
		);
		assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

		assert_events(vec![
			// withdraw from reserve account(for PHA, is bridge account)
			Event::Balances(balances::Event::Withdraw {
				who: Bridge::account_id(),
				amount: 10,
			}),
			// deposit into recipient
			Event::Balances(balances::Event::Deposit {
				who: RELAYER_A,
				amount: 10,
			}),
		]);
	})
}

#[test]
fn simulate_transfer_solochainassets_from_reserve_to_local() {
	new_test_ext().execute_with(|| {
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
		let bridge_asset: crate::pallet_assets_wrapper::XTransferAsset =
			bridge_asset_location.clone().into();
		let r_id: [u8; 32] = bridge_asset.clone().into_rid(src_chainid);
		let amount: u64 = 100;
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

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset,
			0,
			ALICE,
		));

		// setup solo chain for this asset
		assert_ok!(AssetsWrapper::force_setup_solochain(
			Origin::root(),
			0,
			src_chainid,
		));

		assert_eq!(Assets::balance(0, &ALICE), 0);

		// transfer from asset reserve location, would mint asset into ALICE directly
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			alice_location.encode(),
			amount,
			r_id,
		));
		assert_eq!(Assets::balance(0, &ALICE), amount);
		assert_eq!(
			Assets::balance(0, &src_reserve_location.into_account().into()),
			0
		);

		assert_events(vec![
			// mint asset
			Event::Assets(pallet_assets::Event::Issued {
				asset_id: 0,
				owner: ALICE,
				total_supply: amount,
			}),
		]);
	})
}

#[test]
fn simulate_transfer_solochainassets_from_nonreserve_to_local() {
	new_test_ext().execute_with(|| {
		let src_chainid: u8 = 0;
		let para_asset_location = MultiLocation::new(
			1,
			X2(
				Parachain(2000),
				GeneralKey(b"an asset from karura".to_vec()),
			),
		);
		let para_asset: crate::pallet_assets_wrapper::XTransferAsset =
			para_asset_location.clone().into();
		let amount: u64 = 100;
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

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			para_asset.clone(),
			0,
			ALICE,
		));

		// setup solo chain for this asset
		assert_ok!(AssetsWrapper::force_setup_solochain(
			Origin::root(),
			0,
			src_chainid,
		));

		assert_eq!(Assets::balance(0, &ALICE), 0);

		// mint some token to reserve account, simulate the reserve pool
		assert_ok!(Assets::mint(
			Origin::signed(ALICE),
			0,
			src_reserve_location.clone().into_account().into(),
			amount * 2
		));
		assert_eq!(
			Assets::balance(0, &src_reserve_location.clone().into_account().into()),
			amount * 2
		);
		assert_events(vec![
			// mint asset
			Event::Assets(pallet_assets::Event::Issued {
				asset_id: 0,
				owner: src_reserve_location.clone().into_account().into(),
				total_supply: amount * 2,
			}),
		]);

		// transfer from nonreserve location of asset,
		// first: burn asset from source reserve account
		// second: mint asset into recipient
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			alice_location.encode(),
			amount,
			para_asset.into_rid(src_chainid),
		));
		assert_eq!(Assets::balance(0, &ALICE), amount);
		assert_eq!(
			Assets::balance(0, &src_reserve_location.clone().into_account().into()),
			amount
		);

		assert_events(vec![
			// burn asset
			Event::Assets(pallet_assets::Event::Burned {
				asset_id: 0,
				owner: src_reserve_location.into_account().into(),
				balance: amount,
			}),
			// mint asset
			Event::Assets(pallet_assets::Event::Issued {
				asset_id: 0,
				owner: ALICE,
				total_supply: amount,
			}),
		]);
	})
}

#[test]
fn create_successful_transfer_proposal() {
	new_test_ext().execute_with(|| {
		let prop_id = 1;
		let src_id = 1;
		let r_id = NativeTokenResourceId::get();
		let resource = b"BridgeTransfer.transfer".to_vec();
		let relayer_location = MultiLocation::new(
			0,
			X1(AccountId32 {
				network: NetworkId::Any,
				id: RELAYER_A.into(),
			}),
		);
		let proposal = make_transfer_proposal(relayer_location.encode(), 10);

		assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
		assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_C));
		assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));

		// Create proposal (& vote)
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![],
			status: bridge::ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Second relayer votes against
		assert_ok!(Bridge::reject_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A],
			votes_against: vec![RELAYER_B],
			status: bridge::ProposalStatus::Initiated,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		// Third relayer votes in favour
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_C),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone())
		));
		let prop = Bridge::votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
		let expected = bridge::ProposalVotes {
			votes_for: vec![RELAYER_A, RELAYER_C],
			votes_against: vec![RELAYER_B],
			status: bridge::ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);

		assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);
		assert_eq!(
			Balances::free_balance(Bridge::account_id()),
			ENDOWED_BALANCE - 10
		);

		assert_events(vec![
			Event::Bridge(bridge::Event::VoteFor(src_id, prop_id, RELAYER_A)),
			Event::Bridge(bridge::Event::VoteAgainst(src_id, prop_id, RELAYER_B)),
			Event::Bridge(bridge::Event::VoteFor(src_id, prop_id, RELAYER_C)),
			Event::Bridge(bridge::Event::ProposalApproved(src_id, prop_id)),
			// withdraw from reserve account(for PHA, is bridge account)
			Event::Balances(balances::Event::Withdraw {
				who: Bridge::account_id(),
				amount: 10,
			}),
			// deposit into recipient
			Event::Balances(balances::Event::Deposit {
				who: RELAYER_A,
				amount: 10,
			}),
			Event::Bridge(bridge::Event::ProposalSucceeded(src_id, prop_id)),
		]);
	})
}
