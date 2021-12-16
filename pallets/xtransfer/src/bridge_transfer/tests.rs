#![cfg(test)]

use crate::assets_wrapper::pallet::XTransferAssetInfo;
use crate::bridge;
use crate::bridge_transfer::mock::{
	assert_events, balances, expect_event, new_test_ext, Assets, AssetsWrapper, Balances, Bridge,
	BridgeTransfer, Call, Event, NativeTokenResourceId, Origin, ProposalLifetime, Test, ALICE,
	ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use hex_literal::hex;
use sp_runtime::DispatchError;
use xcm::latest::prelude::*;

const TEST_THRESHOLD: u32 = 2;

fn make_transfer_proposal(to: u64, amount: u64) -> Call {
	let resource_id = NativeTokenResourceId::get();
	Call::BridgeTransfer(crate::bridge_transfer::Call::transfer {
		to,
		amount: amount.into(),
		rid: resource_id,
	})
}

#[test]
fn constant_equality() {
	let r_id = bridge::derive_resource_id(1, &bridge::hashing::blake2_128(b"PHA"));
	let encoded: [u8; 32] =
		hex!("00000000000000000000000000000063a7e2be78898ba83824b0c0cc8dfb6001");
	assert_eq!(r_id, encoded);
}

#[test]
fn register_asset() {
	new_test_ext().execute_with(|| {
		let bridge_asset_location = MultiLocation::new(
			1,
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
				GeneralKey(b"an asset".to_vec()),
			),
		);

		// permission denied
		assert_err!(
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
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
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
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain.clone()));
		assert_ok!(BridgeTransfer::change_fee(
			Origin::root(),
			2,
			2,
			dest_chain.clone()
		));

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
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain.clone()));
		assert_ok!(BridgeTransfer::change_fee(
			Origin::root(),
			2,
			2,
			dest_chain.clone()
		));

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
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
fn transfer_assets() {
	new_test_ext().execute_with(|| {
		let dest_chain = 2;
		let bridge_asset_location = MultiLocation::new(
			1,
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain.clone()));
		assert_ok!(BridgeTransfer::change_fee(
			Origin::root(),
			2,
			2,
			dest_chain.clone()
		));

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset_location.clone().into(),
			0,
			ALICE,
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

		// asset has been burned
		assert_eq!(Assets::balance(0, &ALICE), amount);
	})
}

#[test]
fn transfer_native() {
	new_test_ext().execute_with(|| {
		let dest_chain = 0;
		let resource_id = NativeTokenResourceId::get();
		let amount: u64 = 100;
		let recipient = vec![99];

		assert_ok!(Bridge::whitelist_chain(Origin::root(), dest_chain.clone()));
		assert_ok!(BridgeTransfer::change_fee(
			Origin::root(),
			2,
			2,
			dest_chain.clone()
		));

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
	})
}

#[test]
fn simulate_pha_transfer_from_solochain() {
	new_test_ext().execute_with(|| {
		// Check inital state
		let bridge_id: u64 = Bridge::account_id();
		let resource_id = NativeTokenResourceId::get();
		assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
		// Transfer and check result
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			RELAYER_A,
			10,
			resource_id,
		));
		assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
		assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

		assert_events(vec![Event::Balances(balances::Event::Transfer {
			from: Bridge::account_id(),
			to: RELAYER_A,
			amount: 10,
		})]);
	})
}

#[test]
fn simulate_assets_transfer_from_solochain() {
	new_test_ext().execute_with(|| {
		let bridge_asset_location = MultiLocation::new(
			1,
			X3(
				Parachain(2004),
				GeneralKey(b"solochainasset".to_vec()),
				GeneralKey(b"an asset".to_vec()),
			),
		);
		let bridge_asset: crate::pallet_assets_wrapper::XTransferAsset =
			bridge_asset_location.clone().into();
		let r_id: [u8; 32] = bridge_asset.clone().into();
		let amount: u64 = 100;

		// register asset, id = 0
		assert_ok!(AssetsWrapper::force_register_asset(
			Origin::root(),
			bridge_asset,
			0,
			ALICE,
		));

		assert_eq!(Assets::balance(0, &ALICE), 0);
		// transfer to ALICE, would mint asset into ALICE
		assert_ok!(BridgeTransfer::transfer(
			Origin::signed(Bridge::account_id()),
			ALICE,
			amount,
			r_id,
		));
		assert_eq!(Assets::balance(0, &ALICE), amount);
	})
}

#[test]
fn create_successful_transfer_proposal() {
	new_test_ext().execute_with(|| {
		let prop_id = 1;
		let src_id = 1;
		let r_id = bridge::derive_resource_id(src_id, b"transfer");
		let resource = b"BridgeTransfer.transfer".to_vec();
		let proposal = make_transfer_proposal(RELAYER_A, 10);

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
			Event::Balances(balances::Event::Transfer {
				from: Bridge::account_id(),
				to: RELAYER_A,
				amount: 10,
			}),
			Event::Bridge(bridge::Event::ProposalSucceeded(src_id, prop_id)),
		]);
	})
}
