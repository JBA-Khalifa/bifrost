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

#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use codec::{Decode, Encode};
use frame_support::{
	weights::Weight, Parameter, decl_event, decl_error, decl_module,
	decl_storage, debug, ensure, StorageValue, IterableStorageMap,
	traits::{Currency, OnFinalize, Get},
};
use frame_system::{self, ensure_root, ensure_signed};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub type PLOIndex = u32;
pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;
pub type BlockNumberOf<T> = <T as frame_system::Trait>::BlockNumber;
pub type PLOInfoOf<T> = PLOInfo<AccountIdOf<T>, BalanceOf<T>, BlockNumberOf<T>>;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
	type MinContribution: Get<BalanceOf<Self>>;
	type WeightInfo: WeightInfo;
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct PLOInfo<AccountId, Balance, BlockNumber> {
	pub owner: AccountId,
	pub plo_index: PLOIndex,
	pub goal: Balance,
	pub deposit: Balance,
	pub ghost: PhantomData<BlockNumber>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum PLOStatus {
	Success,
	Failure,
	Funding,
}

impl Default for PLOStatus {
	fn default() -> Self {
		Self::Funding
	}
}

pub trait WeightInfo {
	fn create_plo_fund() -> Weight;
}

impl WeightInfo for () {
	fn create_plo_fund() -> Weight { Default::default() }
}

decl_event! {
	pub enum Event<T>
		where AccountId = <T as frame_system::Trait>::AccountId,
			  Balance = BalanceOf<T>,
	{
		/// A new contributor comes to this PLO fund
		NewContributor(AccountId, Balance, PLOIndex),
		/// This PLO reach its goal, (owner, plo_index)
		PLOFunded(AccountId, PLOIndex),
		/// A new PLO is created
		CreatedNewPLOFund(AccountId, PLOIndex),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// This plo index is invalid
		InValidPLOIndex,
		/// Less than the smallest MinContribution
		ContributionTooSmall,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as CrowdFund {
		/// Latest plo index and it's not in use.
		PLOCount get(fn plo_count): PLOIndex = 0u32;
		/// Get plo index by user
		FunderPLOIndex get(fn funder_plo_index): map hasher(blake2_128_concat) AccountIdOf<T> => PLOIndex;
		/// Get PLO information by plo index
		PLOFunds get(fn plo_funds): map hasher(blake2_128_concat) PLOIndex => PLOInfoOf<T>;
		/// Check which PLO is funded or not
		PLOFundStatus get(fn is_funded): map hasher(blake2_128_concat) PLOIndex => PLOStatus;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const MinContribution: BalanceOf<T> = T::MinContribution::get();

		fn deposit_event() = default;

		#[weight = T::WeightInfo::create_plo_fund()]
		fn create_plo_fund(origin, goal: BalanceOf<T>) {
			let creator = ensure_signed(origin)?;

			ensure!(goal >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);

			let current_plo_index = PLOCount::get();

			let plo_info = PLOInfo {
				owner: creator.clone(),
				plo_index: current_plo_index,
				goal,
				deposit: 0u32.into(),
				ghost: PhantomData::<T::BlockNumber>,
			};

			PLOCount::mutate(|index| {
				*index = index.saturating_add(1);
			});

			FunderPLOIndex::<T>::insert(&creator, current_plo_index);

			PLOFundStatus::insert(current_plo_index, PLOStatus::default());

			Self::deposit_event(RawEvent::CreatedNewPLOFund(creator, current_plo_index));
		}

		#[weight = T::WeightInfo::create_plo_fund()]
		fn contribute(origin, amount: BalanceOf<T>) {
			let creator = ensure_signed(origin)?;

			todo!();
		}

		#[weight = T::WeightInfo::create_plo_fund()]
		fn withdraw(origin) {
			todo!()
		}
	}
}