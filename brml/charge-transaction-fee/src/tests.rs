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
use frame_support::assert_ok;
use node_primitives::AssetId;

#[test]
fn set_user_fee_charge_order_should_work() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let origin_signed = Origin::signed(alice);
        let asset_id_0: AssetId = 0;
        let asset_id_1: AssetId = 1;
        let asset_id_2: AssetId = 2;
        let asset_id_3: AssetId = 3;
        let asset_id_4: AssetId = 4;
        let asset_order_list_vec = vec![asset_id_4, asset_id_3, asset_id_2, asset_id_1, asset_id_0];
        assert_ok!(ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed,
            Some(asset_order_list_vec)
        ));
    });
}

// fn inner_get_user_fee_charge_order_list_should_work() {
//     new_test_ext().execute_with(|| {});
// }

// fn ensure_can_charge_fee_should_work() {
//     new_test_ext().execute_with(|| {});
// }

// fn withdraw_fee_should_work() {
//     new_test_ext().execute_with(|| {});
// }

// fn correct_and_deposit_fee_should_work() {
//     new_test_ext().execute_with(|| {});
// }
