// Copyright 2019-2020 Liebi Technologies.
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

#![cfg(test)]

use frame_support::{
	impl_outer_event, impl_outer_origin, parameter_types,
	traits::{LockIdentifier, OnFinalize, OnInitialize},
};
use frame_system as system;
use pallet_balances as balances;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		balances<T>,
		brml_crowd_fund<T>,
		system<T>,
	}
}

mod brml_crowd_fund {
	pub use crate::Event;
}

pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const MaxLocks: u32 = 1024;
}

impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = AccountIndex;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type AccountData = balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type BaseCallFilter = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type SystemWeightInfo = ();
	type PalletInfo = ();
}

impl balances::Trait for Test {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type Event = TestEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const CrowdFundModuleId: LockIdentifier = *b"plofund ";
	pub const MinContribution: u64 = 10;
	pub const MinStakedBNC: u64 = 10;
}

impl crate::Trait for Test {
	type Event = TestEvent;
	type Currency = Balances;
	type AssetCurrency = u32;
	type ModuleId = CrowdFundModuleId;
	type MinContribution = MinContribution;
	type MinStakedBNC = MinStakedBNC;
	type WeightInfo = ();
}

pub type Balances = balances::Module<Test>;
pub type CrowdFund = crate::Module<Test>;
pub type System = system::Module<Test>;

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		CrowdFund::on_initialize(System::block_number());
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}
