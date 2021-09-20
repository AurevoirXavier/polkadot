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

use super::*;
use crate::configuration::HostConfiguration;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use primitives::v1::{HeadData, Id as ParaId, ValidationCode, MAX_CODE_SIZE, MAX_HEAD_DATA_SIZE};
use sp_runtime::traits::{One, Saturating};

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn generate_disordered_pruning<T: Config>() {
	let mut needs_pruning = Vec::new();

	for i in 0..1000 {
		let id = ParaId::from(i);
		let block_number = T::BlockNumber::from(1000u32);
		needs_pruning.push((id, block_number));
	}

	<Pallet<T> as Store>::PastCodePruning::put(needs_pruning);
}

benchmarks! {
	force_set_current_code {
		let c in 1 .. MAX_CODE_SIZE;
		let new_code = ValidationCode(vec![0; c as usize]);
		let para_id = ParaId::from(c as u32);
		generate_disordered_pruning::<T>();
	}: _(RawOrigin::Root, para_id, new_code)
	verify {
		assert_last_event::<T>(Event::CurrentCodeUpdated(para_id).into());
	}
	force_set_current_head {
		let new_head = HeadData(vec![0; MAX_HEAD_DATA_SIZE as usize]);
		let para_id = ParaId::from(1000);
	}: _(RawOrigin::Root, para_id, new_head)
	verify {
		assert_last_event::<T>(Event::CurrentHeadUpdated(para_id).into());
	}
	force_schedule_code_upgrade {
		let c in 1 .. MAX_CODE_SIZE;
		let new_code = ValidationCode(vec![0; c as usize]);
		let para_id = ParaId::from(c as u32);
		let block = T::BlockNumber::from(c);
	}: _(RawOrigin::Root, para_id, new_code, block)
	verify {
		assert_last_event::<T>(Event::CodeUpgradeScheduled(para_id).into());
	}
	force_note_new_head {
		let para_id = ParaId::from(1000);
		let new_head = HeadData(vec![0; MAX_HEAD_DATA_SIZE as usize]);
		// schedule an expired code upgrade for this para_id so that force_note_new_head would use
		// the worst possible code path
		let expired = frame_system::Pallet::<T>::block_number().saturating_sub(One::one());
		let config = HostConfiguration::<T::BlockNumber>::default();
		generate_disordered_pruning::<T>();
		Pallet::<T>::schedule_code_upgrade(para_id, ValidationCode(vec![0]), expired, &config);
	}: _(RawOrigin::Root, para_id, new_head)
	verify {
		assert_last_event::<T>(Event::NewHeadNoted(para_id).into());
	}
	force_queue_action {
		let para_id = ParaId::from(1000);
	}: _(RawOrigin::Root, para_id)
	verify {
		let next_session = crate::shared::Pallet::<T>::session_index().saturating_add(One::one());
		assert_last_event::<T>(Event::ActionQueued(para_id, next_session).into());
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::mock::new_test_ext(Default::default()),
	crate::mock::Test
);
