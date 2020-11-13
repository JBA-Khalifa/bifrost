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

use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, weights::Weight, StorageValue,
	traits::{Currency, Get, LockIdentifier, LockableCurrency, ReservableCurrency, WithdrawReasons},
};
use frame_system::{self, ensure_signed};
use sp_runtime::traits::{Saturating, Zero};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub type PLOIndex = u32;
pub type BalanceOf<T> =
	<<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;
pub type BlockNumberOf<T> = <T as frame_system::Trait>::BlockNumber;
pub type PLOInfoOf<T> = PLOInfo<AccountIdOf<T>, BalanceOf<T>, BlockNumberOf<T>>;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Currency: ReservableCurrency<Self::AccountId>
		+ LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	type AssetCurrency: core::fmt::Display;
	/// Current module id
	type ModuleId: Get<LockIdentifier>;
	type MinContribution: Get<BalanceOf<Self>>;
	type MinStakedBNC: Get<BalanceOf<Self>>;
	type WeightInfo: WeightInfo;
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct PLOInfo<AccountId, Balance, BlockNumber> {
	/// The creator of PLO
	pub owner: AccountId,
	/// PLO index
	pub plo_index: PLOIndex,
	/// PLO creator need to stake their token in bifrost
	pub staked_bnc: Balance,
	/// The whole balance to fund the PLO
	pub goal: Balance,
	/// Already deposited
	pub deposit: Balance,
	/// The PLO status
	pub status: PLOStatus,
	pub ghost: PhantomData<BlockNumber>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum PLOStatus {
	/// Action parachain slot successfully
	Success,
	/// Fail to action parachain slot
	Failure,
	/// The PLO need nore fund
	Funding,
	/// Funded on Bifrost
	Funded,
	/// The PLO has been expired
	Expired,
	/// The funder discard the PLO
	Discarded,
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
	fn create_plo_fund() -> Weight {
		Default::default()
	}
	fn contribute() -> Weight {
		Default::default()
	}
	fn discard() -> Weight {
		Default::default()
	}
	fn withdraw() -> Weight {
		Default::default()
	}
}

decl_event! {
	pub enum Event<T>
		where AccountId = <T as frame_system::Trait>::AccountId,
			  Balance = BalanceOf<T>,
	{
		/// A new contributor comes to this PLO fund
		ContributedPLO(AccountId, Balance, PLOIndex),
		/// A new PLO is created
		CreatedNewPLOFund(AccountId, PLOIndex),
		/// The PLO is funded
		PLOFunded(PLOIndex),
		/// The PLO is discarded
		PLODiscarded(AccountId, PLOIndex),
		/// The investor withdraw their DOT
		Withdraw(AccountId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// This PLO index is invalid
		InValidPLOIndex,
		/// Less than the smallest MinContribution
		ContributionTooSmall,
		/// Stake too small BNC for this PLO
		StakeTooSmallBNC,
		/// The funder hasn't enough balance for staking
		NotEnoughBalanceForStaking,
		/// The investor doesn't invest current PLO
		YouDidNotInvestThisPLO,
		/// Who doesn't own the PLO
		YouDoNotOwnThePLO,
		/// The creator can only create one PLO fund
		MoreThanOnePLOFund,
		/// Already reach the goal
		AlreadyFunded,
		/// The investor invest too much, caused the whole deposit exceeds the PLO goal
		ExceedTheFundGoal,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as CrowdFund {
		/// Latest PLO index and it's not in use.
		PLOCount get(fn plo_count): PLOIndex = 0u32;
		/// Get PLO index by funder
		FunderPLOIndex get(fn funder_plo_index): map hasher(blake2_128_concat) AccountIdOf<T> => PLOIndex;
		/// Get PLO information by PLO index
		PLOFunds get(fn plo_funds): map hasher(blake2_128_concat) PLOIndex => PLOInfoOf<T>;
		/// Check which PLO is funded or not
		PLOFundStatus get(fn is_funded): map hasher(blake2_128_concat) PLOIndex => PLOStatus;
		/// Get PLO index by investor, and who might support multiple PLO funds
		InvestorPLOIndex get(fn investor_plo_index): map hasher(blake2_128_concat) AccountIdOf<T> => Vec<PLOIndex>;
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
		fn create_plo_fund(origin, #[compact] goal: BalanceOf<T>, #[compact] staked_bnc: BalanceOf<T>) {
			let creator = ensure_signed(origin)?;

			ensure!(goal >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);
			ensure!(staked_bnc >= T::MinStakedBNC::get(), Error::<T>::StakeTooSmallBNC);

			let current_plo_index = PLOCount::get();

			let plo_info = PLOInfo {
				owner: creator.clone(),
				plo_index: current_plo_index,
				staked_bnc,
				goal,
				deposit: Zero::zero(),
				status: PLOStatus::default(),
				ghost: PhantomData::<T::BlockNumber>,
			};

			PLOFunds::<T>::insert(current_plo_index, &plo_info);

			PLOCount::mutate(|index| {
				*index = index.saturating_add(1);
			});

			T::Currency::set_lock(T::ModuleId::get(), &creator, staked_bnc, WithdrawReasons::RESERVE);

			FunderPLOIndex::<T>::insert(&creator, current_plo_index);
			PLOFundStatus::insert(current_plo_index, PLOStatus::default());

			Self::deposit_event(RawEvent::CreatedNewPLOFund(creator, current_plo_index));
		}

		/// The investor wants to contribute this PLO
		#[weight = T::WeightInfo::contribute()]
		fn contribute(origin, #[compact] plo_index: PLOIndex, #[compact] amount: BalanceOf<T>) {
			let contributor = ensure_signed(origin)?;

			// check the PLO reach the goal or not
			let plo_info = PLOFunds::<T>::get(plo_index);
			ensure!(
				plo_info.deposit == plo_info.goal && plo_info.status == PLOStatus::Funded,
				Error::<T>::AlreadyFunded
			);
			// ensure investor doesn't invest too much
			ensure!(plo_info.deposit + amount <= plo_info.goal, Error::<T>::ExceedTheFundGoal);

			let dot_balance = T::Currency::free_balance(&contributor);
			// ensure the contributor has enough dot to invest the PLO
			ensure!(amount >= dot_balance, Error::<T>::NotEnoughBalanceForStaking);

			// deposit dot to this PLO
			let mut funded = false;
			PLOFunds::<T>::mutate(plo_index, |plo_info| {
				plo_info.deposit = plo_info.deposit.saturating_add(amount);
				if plo_info.deposit == plo_info.goal {
					plo_info.status = PLOStatus::Funded;
					funded = true;
				}
			});

			// the investor may invest a PLO that s/he has invested.
			if InvestorInfo::<T>::contains_key(&contributor, plo_index) {
				InvestorInfo::<T>::mutate(&contributor, plo_index, |info| {
					*info = info.saturating_add(amount);
				});
			} else {
				// new investor for this PLO
				InvestorInfo::<T>::insert(&contributor, plo_index, amount);
			}

			// the insestors relation to PLO index
			if InvestorPLOIndex::<T>::contains_key(&contributor) {
				InvestorPLOIndex::<T>::mutate(&contributor, |indexes| {
					indexes.push(plo_index);
				});
			} else {
				InvestorPLOIndex::<T>::insert(&contributor, vec![plo_index]);
			}

			if funded {
				Self::deposit_event(RawEvent::PLOFunded(plo_index));
			} else {
				Self::deposit_event(RawEvent::ContributedPLO(contributor, amount, plo_index));
			}

			todo!("
				1. The investor can contribute their dots to the PLO.
				2. When investor transfer dots to PLO funder successfully, bifrost will issue vsdot to investor.
			");
		}

		/// The creator wants to discard this PLO for some reasons
		#[weight = T::WeightInfo::discard()]
		fn discard(origin, #[compact] plo_index: PLOIndex) {
			let who = ensure_signed(origin)?;

			// check the caller own the PLO
			ensure!(FunderPLOIndex::<T>::get(&who) == plo_index, Error::<T>::YouDoNotOwnThePLO);

			// change the status of the PLO to Discarded
			PLOFunds::<T>::mutate(plo_index, |plo_info| {
				plo_info.status = PLOStatus::Discarded;
			});

			Self::deposit_event(RawEvent::PLODiscarded(who, plo_index));

			todo!("
				1. When the IPO funder actions his parachain slot, bifrost should receive this message from relaychain.
				2. Then, bifrost should discard this PLO, I don't know it should be called by funder or do it by bifrost.
				3. Return dots to investors.
			");
		}

		/// investors want to withdraw the dots if fail to auction PLO slot
		#[weight = T::WeightInfo::withdraw()]
		fn withdraw(origin, #[compact] plo_index: PLOIndex) {
			let who = ensure_signed(origin)?;

			// ensure this plo index is created
			ensure!(plo_index < PLOCount::get(), Error::<T>::InValidPLOIndex);
			// ensure investor invests this PLO
			ensure!(InvestorInfo::<T>::contains_key(&who, plo_index), Error::<T>::YouDidNotInvestThisPLO);

			let balance = InvestorInfo::<T>::get(&who, plo_index);

			Self::deposit_event(RawEvent::Withdraw(who, balance));

			todo!("
				1. ensure current PLO expires or discarded by funder, or funder fails to action parachain slot.
				2. Send xcmp message to relaychain, let investor withdraw their dots.
				3. If DOTs are transfered to relaychain, then go destroy vsDOT or vsKS.
				4.
			");
		}
	}
}
