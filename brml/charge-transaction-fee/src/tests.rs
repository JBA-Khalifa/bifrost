// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]

use crate::mock::*;
// use balances::Call as BalancesCall;
use frame_support::{
    assert_ok,
    traits::{Currency, WithdrawReasons},
    weights::{GetDispatchInfo, Pays, PostDispatchInfo},
};
use node_primitives::{AssetId, AssetTrait, TokenType};
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::testing::TestXt;

// some common variables
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const ASSET_ID_0: AssetId = 0;
pub const ASSET_ID_1: AssetId = 1;
pub const ASSET_ID_2: AssetId = 2;
pub const ASSET_ID_3: AssetId = 3;
pub const ASSET_ID_4: AssetId = 4;

fn basic_setup() {
    assert_ok!(Assets::create(
        Origin::root(),
        b"BNC".to_vec(),
        18,
        TokenType::Stable
    )); // Asset Id 0
    assert_ok!(Assets::create(
        Origin::root(),
        b"aUSD".to_vec(),
        18,
        TokenType::Stable
    )); // Asset Id 1
    assert_ok!(Assets::create_pair(Origin::root(), b"DOT".to_vec(), 18)); // Asset Id id 2,3
    assert_ok!(Assets::create_pair(Origin::root(), b"KSM".to_vec(), 18)); // Asset Id id 4,5
    assert_ok!(Assets::create_pair(Origin::root(), b"EOS".to_vec(), 18)); // Asset Id id 6,7
    assert_ok!(Assets::create_pair(Origin::root(), b"IOST".to_vec(), 18)); // Asset Id id 8,9

    // Deposit some money in Alice, Bob and Charlie's accounts.
    // Alice
    let _alice = <Test as crate::Config>::Currency::deposit_creating(&ALICE, 50); // native token
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_1, ALICE, 200));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_2, ALICE, 300));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_3, ALICE, 400));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_4, ALICE, 500));

    // Bob
    let _bob = <Test as crate::Config>::Currency::deposit_creating(&BOB, 100); // native token
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_1, BOB, 200));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_2, BOB, 60));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_3, BOB, 80));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_4, BOB, 50));

    // Charlie
    assert_ok!(Assets::issue(
        Origin::root(),
        ASSET_ID_0,
        CHARLIE,
        300_000_000_000_000_000
    ));

    let _charlie = <Test as crate::Config>::Currency::deposit_creating(&CHARLIE, 200); // native token
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_1, CHARLIE, 20));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_2, CHARLIE, 30));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_3, CHARLIE, 40));
    assert_ok!(Assets::issue(Origin::root(), ASSET_ID_4, CHARLIE, 50));
}

#[test]
fn set_user_fee_charge_order_should_work() {
    new_test_ext().execute_with(|| {
        let origin_signed_alice = Origin::signed(ALICE);
        let mut asset_order_list_vec: Vec<AssetId> =
            vec![ASSET_ID_4, ASSET_ID_3, ASSET_ID_2, ASSET_ID_1, ASSET_ID_0];
        assert_ok!(ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice.clone(),
            Some(asset_order_list_vec.clone())
        ));

        asset_order_list_vec.insert(0, ASSET_ID_0);
        assert_eq!(
            crate::UserFeeChargeOrderList::<Test>::get(ALICE),
            asset_order_list_vec
        );

        assert_ok!(ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice,
            None
        ));

        assert_eq!(
            crate::UserFeeChargeOrderList::<Test>::get(ALICE).is_empty(),
            true
        );
    });
}

#[test]
fn inner_get_user_fee_charge_order_list_should_work() {
    new_test_ext().execute_with(|| {
        let origin_signed_alice = Origin::signed(ALICE);
        let mut asset_order_list_vec: Vec<AssetId> =
            vec![ASSET_ID_4, ASSET_ID_3, ASSET_ID_2, ASSET_ID_1, ASSET_ID_0];

        let mut default_order_list: Vec<AssetId> = Vec::new();
        for i in 0..10 {
            default_order_list.push(AssetId::from(i as u32));
        }
        assert_eq!(
            ChargeTransactionFee::inner_get_user_fee_charge_order_list(&ALICE),
            default_order_list
        );

        let _ = ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice.clone(),
            Some(asset_order_list_vec.clone()),
        );

        asset_order_list_vec.insert(0, ASSET_ID_0);

        assert_eq!(
            ChargeTransactionFee::inner_get_user_fee_charge_order_list(&ALICE),
            asset_order_list_vec
        );
    });
}

#[test]
fn ensure_can_charge_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();
        let origin_signed_bob = Origin::signed(BOB);
        let asset_order_list_vec: Vec<AssetId> =
            vec![ASSET_ID_4, ASSET_ID_3, ASSET_ID_2, ASSET_ID_1, ASSET_ID_0];
        let mut default_order_list: Vec<AssetId> = Vec::new();
        for i in 0..10 {
            default_order_list.push(AssetId::from(i as u32));
        }

        // Set bob order as [4,3,2,1]. Alice and Charlie will use the default order of [0..9]]
        let _ = ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_bob.clone(),
            Some(asset_order_list_vec.clone()),
        );

        ChargeTransactionFee::ensure_can_charge_fee(
            &ALICE,
            100,
            WithdrawReasons::TRANSACTION_PAYMENT,
        );

        // Alice should be deducted 100 from Asset 1 since Asset 0 doesn't have enough balance. asset1 : 200-100=100
        // asset0: 50+100 = 150
        assert_eq!(
            <Test as crate::Config>::AssetTrait::get_account_asset(ASSET_ID_1, &ALICE).balance,
            100
        );

        assert_eq!(<Test as crate::Config>::Currency::free_balance(&ALICE), 150);

        // Bob
        ChargeTransactionFee::ensure_can_charge_fee(
            &BOB,
            100,
            WithdrawReasons::TRANSACTION_PAYMENT,
        );
        assert_eq!(<Test as crate::Config>::Currency::free_balance(&BOB), 100);
        assert_eq!(
            <Test as crate::Config>::AssetTrait::get_account_asset(ASSET_ID_1, &BOB).balance,
            200
        );
    });
}

#[test]
fn withdraw_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();

        // prepare call variable
        let asset_order_list_vec: Vec<AssetId> =
            vec![ASSET_ID_0, ASSET_ID_1, ASSET_ID_2, ASSET_ID_3, ASSET_ID_4];
        let call = Call::ChargeTransactionFee(crate::Call::set_user_fee_charge_order(Some(
            asset_order_list_vec,
        )));

        // prepare info variable
        let extra = ();
        let xt = TestXt::new(call.clone(), Some((CHARLIE, extra)));
        let info = xt.get_dispatch_info();

        // 99 inclusion fee and a tip of 8
        assert_ok!(ChargeTransactionFee::withdraw_fee(
            &CHARLIE, &call, &info, 107, 8
        ));

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            93
        );
    });
}

#[test]
fn correct_and_deposit_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();
        // prepare call variable
        let asset_order_list_vec: Vec<AssetId> =
            vec![ASSET_ID_0, ASSET_ID_1, ASSET_ID_2, ASSET_ID_3, ASSET_ID_4];
        let call = Call::ChargeTransactionFee(crate::Call::set_user_fee_charge_order(Some(
            asset_order_list_vec,
        )));
        // prepare info variable
        let extra = ();
        let xt = TestXt::new(call.clone(), Some((CHARLIE, extra)));
        let info = xt.get_dispatch_info();

        // prepare post info
        let post_info = PostDispatchInfo {
            actual_weight: Some(20),
            pays_fee: Pays::Yes,
        };

        let corrected_fee = 80;
        let tip = 8;

        let already_withdrawn =
            ChargeTransactionFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8).unwrap();

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            93
        );

        assert_ok!(ChargeTransactionFee::correct_and_deposit_fee(
            &CHARLIE,
            &info,
            &post_info,
            corrected_fee,
            tip,
            already_withdrawn
        ));

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            120
        );
    });
}
