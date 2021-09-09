// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Pallet that serves no other purpose than benchmarking raw messages [`Xcm`].

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_benchmarking::{BenchmarkError, BenchmarkResult};
use frame_support::{
	traits::{
		fungible::Inspect as FungibleInspect,
		fungibles::Inspect as FungiblesInspect,
		tokens::{DepositConsequence, WithdrawConsequence},
	},
	weights::Weight,
};
use sp_std::prelude::*;
use xcm::latest::prelude::*;
use xcm_executor::{traits::Convert, Assets};

pub mod fungible;

#[cfg(test)]
mod mock;

/// A base trait for all individual pallets
pub trait Config: frame_system::Config {
	/// The XCM configurations.
	///
	/// These might affect the execution of XCM messages, such as defining how the
	/// `TransactAsset` is implemented.
	type XcmConfig: xcm_executor::Config;

	// temp?
	type AccountIdConverter: Convert<MultiLocation, Self::AccountId>;

	/// Does any necessary setup to create a valid destination for XCM messages.
	/// Returns that destination's multi-location to be used in benchmarks.
	fn valid_destination() -> Result<MultiLocation, sp_runtime::DispatchError>;
}

const SEED: u32 = 0;

/// The xcm executor to use for doing stuff.
pub type ExecutorOf<T> = xcm_executor::XcmExecutor<<T as Config>::XcmConfig>;
/// The overarching call type.
pub type OverArchingCallOf<T> = <T as frame_system::Config>::Call;
/// The asset transactor of our executor
pub type AssetTransactorOf<T> = <<T as Config>::XcmConfig as xcm_executor::Config>::AssetTransactor;
/// The call type of executor's config. Should eventually resolve to the same overarching call type.
pub type XcmCallOf<T> = <<T as Config>::XcmConfig as xcm_executor::Config>::Call;

/// The worst case number of assets in the holding.
const HOLDING_FUNGIBLES: u32 = 99;
const HOLDING_NON_FUNGIBLES: u32 = 99;

pub fn worst_case_holding() -> Assets {
	let fungibles_amount: u128 = 100; // TODO probably update
	(0..HOLDING_FUNGIBLES)
		.map(|i| {
			MultiAsset {
				id: Concrete(GeneralIndex(i as u128).into()),
				fun: Fungible(fungibles_amount * i as u128),
			}
			.into()
		})
		.chain(core::iter::once(MultiAsset { id: Concrete(Here.into()), fun: Fungible(u128::MAX) }))
		.chain((0..HOLDING_NON_FUNGIBLES).map(|i| MultiAsset {
			id: Concrete(GeneralIndex(i as u128).into()),
			fun: NonFungible(asset_instance_from(i)),
		}))
		.collect::<Vec<_>>()
		.into()
}

pub fn asset_instance_from(x: u32) -> AssetInstance {
	let bytes = x.encode();
	let mut instance = [0u8; 4];
	instance.copy_from_slice(&bytes);
	AssetInstance::Array4(instance)
}

/// Execute an xcm.
/// TODO: This skips all the barriers and traders, etc... maybe need to add back.
pub fn execute_xcm<T: Config>(
	origin: MultiLocation,
	holding: Assets,
	xcm: Xcm<XcmCallOf<T>>,
) -> Result<(), BenchmarkError> {
	// TODO: very large weight to ensure all benchmarks execute, sensible?
	let mut executor = ExecutorOf::<T>::new(origin);
	executor.holding = holding;
	executor.execute(xcm)?;
	Ok(())
}

pub fn execute_xcm_override_error<T: Config>(
	origin: MultiLocation,
	holding: Assets,
	xcm: Xcm<XcmCallOf<T>>,
) -> Result<(), BenchmarkError> {
	execute_xcm::<T>(origin, holding, xcm)
		.map_err(|_| BenchmarkError::Override(BenchmarkResult::from_weight(Weight::MAX)))
}

pub fn new_executor<T: Config>(origin: MultiLocation) -> ExecutorOf<T> {
	ExecutorOf::<T>::new(origin)
}

// TODO probably delete and use converter
pub fn account<T: frame_system::Config>(index: u32) -> T::AccountId {
	frame_benchmarking::account::<T::AccountId>("account", index, SEED)
}

/// Build a multi-location from an account id.
fn account_id_junction<T: frame_system::Config>(index: u32) -> Junction {
	let account = account::<T>(index);
	let mut encoded = account.encode();
	encoded.resize(32, 0u8);
	let mut id = [0u8; 32];
	id.copy_from_slice(&encoded);
	Junction::AccountId32 { network: NetworkId::Any, id }
}

pub fn account_and_location<T: Config>(index: u32) -> (T::AccountId, MultiLocation) {
	let location: MultiLocation = account_id_junction::<T>(index).into();
	let account = T::AccountIdConverter::convert(location.clone()).unwrap();

	(account, location)
}

/// Helper struct that converts a `Fungible` to `Fungibles`
///
/// TODO: might not be needed anymore.
pub struct AsFungibles<AccountId, AssetId, B>(sp_std::marker::PhantomData<(AccountId, AssetId, B)>);
impl<
		AccountId: sp_runtime::traits::Member + frame_support::dispatch::Parameter,
		AssetId: sp_runtime::traits::Member + frame_support::dispatch::Parameter + Copy,
		B: FungibleInspect<AccountId>,
	> FungiblesInspect<AccountId> for AsFungibles<AccountId, AssetId, B>
{
	type AssetId = AssetId;
	type Balance = B::Balance;

	fn total_issuance(_: Self::AssetId) -> Self::Balance {
		B::total_issuance()
	}
	fn minimum_balance(_: Self::AssetId) -> Self::Balance {
		B::minimum_balance()
	}
	fn balance(_: Self::AssetId, who: &AccountId) -> Self::Balance {
		B::balance(who)
	}
	fn reducible_balance(_: Self::AssetId, who: &AccountId, keep_alive: bool) -> Self::Balance {
		B::reducible_balance(who, keep_alive)
	}
	fn can_deposit(_: Self::AssetId, who: &AccountId, amount: Self::Balance) -> DepositConsequence {
		B::can_deposit(who, amount)
	}

	fn can_withdraw(
		_: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		B::can_withdraw(who, amount)
	}
}
