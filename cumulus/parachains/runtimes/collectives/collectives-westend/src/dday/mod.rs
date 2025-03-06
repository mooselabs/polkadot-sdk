// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! New governance configurations for the D-Day parachain rescue (primary AssetHub) scenario.

mod tracks;
pub mod prover;

use super::*;
use super::fellowship::{Architects, FellowshipCollectiveInstance, Masters, ranks};
use frame_support::parameter_types;
use frame_support::traits::{ConstU128, EitherOf, PollStatus, Polling};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_referenda::ReferendumIndex;
use sp_runtime::DispatchError;
use crate::dday::tracks::TrackId;

// TODO: FAIL-CI - check constants
parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 1 * 3 * CENTS;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

/// Wrapper implementation of `Polling` over `DDayReferenda` polling.
/// TODO: FAIL-CI - maybe not needed at the end?
pub struct AllowPollingWhenStalled;
impl Polling<pallet_referenda::TallyOf<Runtime, DDayReferendaInstance>> for AllowPollingWhenStalled {
	type Index = ReferendumIndex;
	type Votes = pallet_referenda::VotesOf<Runtime, DDayReferendaInstance>;
	type Class = TrackId;
	type Moment = BlockNumberFor<Runtime>;

	fn classes() -> Vec<Self::Class> {
		DDayReferenda::classes()
	}

	fn as_ongoing(index: Self::Index) -> Option<(pallet_referenda::TallyOf<Runtime, DDayReferendaInstance>, Self::Class)> {
		// TODO: check stalled AssetHub's state_root
		DDayReferenda::as_ongoing(index)
	}

	fn access_poll<R>(
		index: Self::Index,
		f: impl FnOnce(PollStatus<&mut pallet_referenda::TallyOf<Runtime, DDayReferendaInstance>, Self::Moment, Self::Class>) -> R,
	) -> R {
		// TODO: check stalled AssetHub's state_root
		DDayReferenda::access_poll(index, f)
	}

	fn try_access_poll<R>(
		index: Self::Index,
		f: impl FnOnce(
			PollStatus<&mut pallet_referenda::TallyOf<Runtime, DDayReferendaInstance>, Self::Moment, Self::Class>,
		) -> Result<R, DispatchError>,
	) -> Result<R, DispatchError> {
		// TODO: check stalled AssetHub's state_root
		DDayReferenda::try_access_poll(index, f)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_ongoing(class: Self::Class) -> Result<Self::Index, ()> {
		DDayReferenda::create_ongoing(class)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn end_ongoing(index: Self::Index, approved: bool) -> Result<(), ()> {
		DDayReferenda::end_ongoing(index, approved)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn max_ongoing() -> (Self::Class, u32) {
		DDayReferenda::max_ongoing()
	}
}

/// Setup voting by AssetHub account proofs.
pub type DDayVotingInstance = pallet_proofs_voting::Instance1;
impl pallet_proofs_voting::Config<DDayVotingInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// TODO: FAIL-CI - setup/generate benchmarks
	type WeightInfo = pallet_proofs_voting::weights::SubstrateWeight<Self>;
	type Polls = AllowPollingWhenStalled;
	// TODO: FAIL-CI - we have `Balances::total_issuance` as a part of every proof dynamically?
	// TotalIssuanceOf<Balances? Maybe adjust it dynamically per state_root from every vote,
	// it should be the same for every vote and the same state_root.
	type MaxTurnout = ConstU128<{ u128::MAX }>;
	type MaxVotes = ConstU32<3>;
	type BlockNumberProvider = System;

	type Prover = prover::AssetHubAccountProver;
	type ProofRootProvider = prover::AssetHubStateRootProvider;
}

/// Rank3+ member can start DDay referendum.
pub type DDayReferendaInstance = pallet_referenda::Instance3;
impl pallet_referenda::Config<DDayReferendaInstance> for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_referenda_dday::WeightInfo<Self>;
	type Scheduler = Scheduler;
	type Currency = Balances;
	/// Only rank3+ can start referendum
	type SubmitOrigin =	pallet_ranked_collective::EnsureMember<
		Runtime,
		FellowshipCollectiveInstance,
		{ ranks::DAN_3 }
	>;
	/// Only rank4+ can cancel/kill referendum
	type CancelOrigin = EitherOf<Architects, Masters>;
	type KillOrigin = EitherOf<Architects, Masters>;
	type Slash = ToParentTreasury<WestendTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = pallet_proofs_voting::VotesOf<Runtime, DDayVotingInstance>;
	type Tally = pallet_proofs_voting::TallyOf<Runtime, DDayVotingInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<2>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}
