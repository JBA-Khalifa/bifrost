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
#![cfg_attr(not(feature = "std"), no_std)]

/// All the new rules to write a pallet, visit this link:
/// https://crates.parity.io/frame_support/attr.pallet.html#upgrade-guidelines
/// There're more detailed information in this link.

mod mock;
mod tests;

pub use self::your_pallet_name::*;

#[frame_support::pallet]
pub mod your_pallet_name {
	use core::marker::PhantomData;
	use frame_support::traits::{Get, Hooks, IsType};
	use frame_support::{Parameter, transactional};
	use frame_support::weights::Weight;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::{OriginFor, BlockNumberFor};
	use frame_system::{ensure_root, ensure_signed};
	use node_primitives::{PalletStorageVersion};
	use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};
	
	type BalanceOf<T> = <T as Config>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The units in which we record balances.
		type Balance: Member 
			+ Parameter
			+ AtLeast32Bit
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ From<Self::BlockNumber>
			+ Into<Self::MintPrice>;
		/// Add your more type here.

		#[pallet::constant]
		type Constant: Get<Self::BlockNumber>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Explain what this error is
		Error1,
		/// Also explain what this error is
		Error2,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(BalanceOf<T> = "Balance", u32 = "u32")]
	pub enum Event<T: Config> {
		/// Explain what this event is
		Event1,
		/// Explain what this event is
		Event2(BalanceOf<T>),
		/// Explain what this event is
		Event3(u32),
	}
	
	/// Single value storage
	/// Explain what this storage stores
	#[pallet::storage]
	#[pallet::getter(fn all_referer_channels)]
	pub(crate) type ValueStorage<T: Config> = StorageValue<
		_,
		T::Balance, // This is the type you want to store, replace your type here: T::YourType
		ValueQuery, // If not exist, return a default value. If it's OptionQuery, return Some(T::YourType) or None.
		()
	>;

	/// Explain what this storage stores
	#[pallet::storage]
	#[pallet::getter(fn all_referer_channels)]
	pub(crate) type StorageVersion<T: Config> = StorageValue<
		_,
		// This represents the current storage version of pallet.
		// You need to update version after every storage migration.
		PalletStorageVersion,
		ValueQuery, // If not exist, return a default value. If it's OptionQuery, return Some(T::YourType) or None.
		()
	>;

	/// key-value storage
	/// Explain what this storage stores
	#[pallet::storage]
	#[pallet::getter(fn mint_price)]
	pub(crate) type 1KeyValue<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId, // Key
		T::Balance, // Value
		ValueQuery
	>;

	/// Explain what this storage stores
	/// Dulble-key, single-value storage
	#[pallet::storage]
	#[pallet::getter(fn referrer_channels)]
	pub(crate) type 2KeysValue<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		T::AccountId,
		T::Balance,
		ValueQuery
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// A couple of hooks here, it depends on your needs.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Explain what on_initalize function is going to do.
		fn on_initalize(block_number: T::BlockNumber) -> Weight {
			todo!("
				1. If you don't need it, just remove it.
				2. If you have your own OnInitialize impl, implment it here.
			");
		}

		/// Explain what on_finalize function is going to do.
		fn on_finalize(block_number: T::BlockNumber) {
			todo!("
				1. If you don't need it, just remove it.
				2. If you have your own OnFinalize impl, implment it here.
			");
		}
		
		/// Explain what offchain_worker function is going to do.
		fn offchain_worker(block_number: T::BlockNumber) {
			todo!("
				1. If you don't need it, just remove it.
				2. If you have your own OffchainWorker impl, implment it here.
			");
		}

		/// Explain how to do storage migration.
		fn on_runtime_upgrade() -> Weight {
			todo!("
				1. If you don't need it, just remove it.
				2. If you have your own OnRuntimeUpgrade impl, implment it here.
			");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// What this call is going to do.
		///
		/// Parameters:
		/// - `amount`: ?.
		/// - `who`: ?.
		#[pallet::weight(T::WeightInfo::your_call())]
		#[transactional]
		pub fn your_call(
			origin: OriginFor<T>,
			#[pallet::compact] amount: T::Balance,
			who: T::AccountId
		) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			
			todo!("
				Implement you call.
			")

			Self::deposit_event(Event::Event1);

			Ok(().into())
		}

	impl<T: Config> Pallet<T> {
		fn your_pallet_method() {
			todo!("Impl your pallet method");
		}
	}
	
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub pool: BalanceOf<T>, // This field is just example.
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { pool: 0u32.into() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			todo!("
				Intialize your pallet genesis storage here.
			")
		}
	}

	pub trait WeightInfo {
		fn your_call<T: Config>() -> Weight;
	}
	
	impl WeightInfo for () {
		fn your_call() -> Weight { Default::default() }
	}
}
