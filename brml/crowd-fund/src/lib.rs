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
	weights::Weight, decl_event, decl_error, decl_module, decl_storage, ensure, StorageValue,
	traits::{Currency, LockableCurrency, ReservableCurrency, Get, LockIdentifier, WithdrawReasons},
};
use frame_system::{self, ensure_signed};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub type PLOIndex = u32;
pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;
pub type BlockNumberOf<T> = <T as frame_system::Trait>::BlockNumber;
pub type PLOInfoOf<T> = PLOInfo<AccountIdOf<T>, BalanceOf<T>, BlockNumberOf<T>>;
const CROWDFUND_ID: LockIdentifier = *b"plofund ";

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Currency: ReservableCurrency<Self::AccountId> + LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
	type ModuleId: Get<LockIdentifier>;
	type MinContribution: Get<BalanceOf<Self>>;
	type MinStakedBNC: Get<BalanceOf<Self>>;
	type WeightInfo: WeightInfo;
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct PLOInfo<AccountId, Balance, BlockNumber> {
	pub owner: AccountId,
	pub plo_index: PLOIndex,
	pub staked_bnc: Balance,
	pub goal: Balance,
	pub deposit: Balance,
	pub ghost: PhantomData<BlockNumber>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum PLOStatus {
	Success,
	Failure,
	Funding,
	Expired,
	Discard,
}

impl Default for PLOStatus {
	fn default() -> Self {
		Self::Funding
	}
}

pub trait WeightInfo {
	fn create_plo_fund() -> Weight;
	fn contribute() -> Weight;
	fn discard() -> Weight;
	fn withdraw() -> Weight;
}

impl WeightInfo for () {
	fn create_plo_fund() -> Weight { Default::default() }
	fn contribute() -> Weight { Default::default() }
	fn discard() -> Weight { Default::default() }
	fn withdraw() -> Weight { Default::default() }
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
		/// Stake too small BNC for this PLO
		StakeTooSmallBNC,
		/// The funder hasn't enough balance for staking
		NotEnoughBalanceForStaking,
		/// The investor doesn't invest current PLO
		YouDidNotInvestThisPLO,
		/// The creator can only create one PLO fund
		MoreThanOnePLOFund,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as CrowdFund {
		/// Latest plo index and it's not in use.
		PLOCount get(fn plo_count): PLOIndex = 0u32;
		/// Get plo index by funder, and who might need multiple PLO to support his slot action
		FunderPLOIndex get(fn funder_plo_index): map hasher(blake2_128_concat) AccountIdOf<T> => Vec<PLOIndex>;
		/// Get PLO information by plo index
		PLOFunds get(fn plo_funds): map hasher(blake2_128_concat) PLOIndex => PLOInfoOf<T>;
		/// Check which PLO is funded or not
		PLOFundStatus get(fn is_funded): map hasher(blake2_128_concat) PLOIndex => PLOStatus;
		/// Record the investor's balance and the relation to PLO index
		InvestorInfo get(fn investor_info):
			double_map hasher(blake2_128_concat) AccountIdOf<T>, hasher(blake2_128_concat) PLOIndex=> BalanceOf<T>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const MinContribution: BalanceOf<T> = T::MinContribution::get();
		const MinStakedBNC: BalanceOf<T> = T::MinStakedBNC::get();
		const ModuleId: LockIdentifier = T::ModuleId::get();

		fn deposit_event() = default;

		#[weight = T::WeightInfo::create_plo_fund()]
		fn create_plo_fund(origin, goal: BalanceOf<T>, staked_bnc: BalanceOf<T>) {
			let creator = ensure_signed(origin)?;

			ensure!(goal >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);
			ensure!(staked_bnc >= T::MinStakedBNC::get(), Error::<T>::StakeTooSmallBNC);

			let bnc_balance = T::Currency::free_balance(&creator);
			ensure!(staked_bnc >= bnc_balance, Error::<T>::NotEnoughBalanceForStaking);

			let current_plo_index = PLOCount::get();

			let plo_info = PLOInfo {
				owner: creator.clone(),
				plo_index: current_plo_index,
				staked_bnc,
				goal,
				deposit: 0u32.into(),
				ghost: PhantomData::<T::BlockNumber>,
			};

			PLOCount::mutate(|index| {
				*index = index.saturating_add(1);
			});

			T::Currency::set_lock(T::ModuleId::get(), &creator, bnc_balance, WithdrawReasons::RESERVE);

			FunderPLOIndex::<T>::insert(&creator, vec![current_plo_index]);

			PLOFundStatus::insert(current_plo_index, PLOStatus::default());

			Self::deposit_event(RawEvent::CreatedNewPLOFund(creator, current_plo_index));
		}

		/// The investor wants to contribute this PLO for some reasons
		#[weight = T::WeightInfo::contribute()]
		fn contribute(origin, plo_index: PLOIndex) {
			let contributor = ensure_signed(origin)?;

			todo!("
				1. The investor can contribute their dots to the PLO.
				2. When investor transfer dots to PLO funder successfully, bifrost will issue vsdot to investor.
			");
		}

		/// The creator wants to discard this PLO for some reasons
		#[weight = T::WeightInfo::discard()]
		fn discard(origin, plo_index: PLOIndex) {
			let destroyer = ensure_signed(origin)?;

			todo!("
				1. When the IPO funder actions his parachain slot, bifrost should receive this message from relaychain.
				2. Then, bifrost should discard this PLO, I don't know it should be called by funder or do it by bifrost.
				3. Return dots to investors.
			");
		}

		/// investors want to withdraw the dots if fail to auction PLO slot
		#[weight = T::WeightInfo::withdraw()]
		fn withdraw(origin, plo_index: PLOIndex) {
			let who = ensure_signed(origin)?;

			// ensure this plo index is created
			ensure!(plo_index < PLOCount::get(), Error::<T>::InValidPLOIndex);
			// ensure investor invests this PLO
			ensure!(InvestorInfo::<T>::contains_key(&who, plo_index), Error::<T>::YouDidNotInvestThisPLO);

			let balance = InvestorInfo::<T>::get(&who, plo_index);

			todo!("
				1. Send xcmp message to relaychain, let investor withdraw their dots.
				2. If DOTs are transfered to relaychain, then go destroy vsDOT or vsKS.
				3.
			");
		}
	}
}