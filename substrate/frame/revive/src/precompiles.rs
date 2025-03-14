// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Exposes types that can be used to `pallet_revive` with additional functionality.
//!
//! Use `alloy` through our re-ep

mod builtin;

#[cfg(test)]
mod tests;

pub use crate::{
	exec::{PrecompileExt as Ext, PrecompileWithInfoExt as ExtWithInfo},
	gas::GasMeter,
	storage::meter::Diff,
};
pub use alloy_core as alloy;

use crate::{exec::ExecResult, precompiles::builtin::Builtin, primitives::ExecReturnValue, Config};
use alloc::vec::Vec;
use alloy::sol_types::{Panic, PanicKind, Revert, SolError, SolInterface};
use core::num::NonZero;
use pallet_revive_uapi::ReturnFlags;

const UNIMPLEMENTED: &str = "A precompile must either implement `call` or `call_with_info`";

/// The composition of all available pre-compiles.
///
/// This is how the rest of the pallet discovers and calls pre-compiles.
pub(crate) type All<T> = (Builtin<T>, <T as Config>::Precompiles);

/// Used by [`Precompile`] in order to declare at which addresses it will be called.
///
/// The 4 byte integer supplied here will be interpreted as big endian and copied to
/// `address[12..16]`. Address `address[16..20]` is reserved for builtin precompiles. All other
/// bytes are set to zero.
///
/// Big endian is chosen because it lines up with how you would invoke a pre-compile in Solidity.
/// For example writing `staticcall(..., 0x05, ...)` in Solidity sets the highest (`address[19]`)
/// byte to `5`.
pub enum AddressMatcher {
	/// The pre-compile will only be called for a single address.
	///
	/// This means the precompile will only be invoked for:
	/// ```
	/// 000000000000000000000000pppppppp00000000
	/// ```
	///
	/// Where `p` is the `u32` defined here as big endian.
	Fixed(NonZero<u32>),
	/// The pre-compile will be called for multiple addresses.
	///
	/// This is useful when some information should be encoded into the address.
	///
	/// This means the precompile will be invoked for all `x`:
	/// ```
	/// xxxxxxxx0000000000000000pppppppp00000000
	/// ```
	///
	/// Where `p` is the `u32` defined here as big endian. Hence a maximum of 4 byte can be encoded
	/// into the address. Allowing more bytes could lead to the situation where legitimate
	/// accounts could exist at this address. Either by accident or on purpose.
	Prefix(NonZero<u32>),
}

/// Same as `AddressMatcher` but for builtin pre-compiles.
///
/// It works in the same way as `AddressMatcher` but allows setting the full 8 byte prefix.
/// Builtin pre-compiles must only use values `<= u32::MAX` to prevent collisions with
/// external pre-compiles.
pub(crate) enum BuiltinAddressMatcher {
	Fixed(NonZero<u64>),
	Prefix(NonZero<u64>),
}

/// Type that can be implemented in other crates to extend the list of pre-compiles.
///
/// Only implement exacly one function. Either `call` or `call_with_info`.
pub trait Precompile {
	/// Your runtime.
	type T: Config;
	/// The Solidity ABI definition of this pre-compile.
	///
	/// Use the [`alloy::sol`] macro to define your interface using Solidity syntax.
	/// The input the caller passes to the pre-compile will be validated and parsed
	/// according to this interface.
	///
	/// Please note that the return value is not validated and it is the pre-compiles
	/// duty to return the abi encoded bytes conformant with the interface here.
	type Interface: SolInterface;
	/// Defines at which addresses this pre-compile exists.
	const MATCHER: AddressMatcher;
	/// Defines whether this pre-compile needs a contract info data structure in storage.
	///
	/// Enabling it unlocks more APIs for the pre-compile to use.
	///
	/// # When set to **true**
	///
	/// - An account will be created at the pre-compiles address when it is called for the first
	///   time. The ed is minted.
	/// - Contract info dats structure will be created in storage on first call.
	/// - Only `call_with_info` should be implemented. `call` is never called.
	///
	/// # When set to **false**
	///
	/// - No account or any other state will be created for the address.
	/// - Only `call` should be implemented. `call_with_info` is never called.
	///
	/// # What to use
	///
	/// Should be set to false if the additional functionality is not needed. A pre-compile with
	/// contract info will incur both a storage read and write to its contract metadata when called.
	///
	/// The contract info enables additional functionality:
	/// - Storage deposits: Collect deposits from the origin rather than the caller. This makes it
	///   easier for contracts to interact with your pre-compile as deposits
	/// 	are payed by the transaction signer (just like gas). It also makes refunding easier.
	/// - Contract storage: You can use the contracts key value child trie storage instead of
	///   providing your own state.
	/// 	The contract storage automatically takes care of deposits.
	/// 	Providing your own storage and using pallet_revive to collect deposits is also possible,
	/// though.
	///
	/// Have a look at [`ExtWithInfo`] to learn about the additional APIs that a contract info
	/// unlocks.
	const HAS_CONTRACT_INFO: bool;

	/// Entry point for your pre-compile when `HAS_CONTRACT_INFO = false`.
	#[allow(unused_variables)]
	fn call(
		address: &[u8; 20],
		input: &Self::Interface,
		env: &impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		unimplemented!("{UNIMPLEMENTED}")
	}

	/// Entry point for your pre-compile when `HAS_CONTRACT_INFO = true`.
	#[allow(unused_variables)]
	fn call_with_info(
		address: &[u8; 20],
		input: &Self::Interface,
		env: &impl ExtWithInfo<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		unimplemented!("{UNIMPLEMENTED}")
	}
}

/// Same as `Precompile` but meant to be used by builtin pre-compiles.
///
/// This enabled builtin precompiles to exist at the highest bits. Those are not
/// available to external pre-compiles in order to avoid collisions.
///
/// Automatically implemented for all types that implement `Precompile`.
pub(crate) trait BuiltinPrecompile {
	type T: Config;
	type Interface: SolInterface;
	const MATCHER: BuiltinAddressMatcher;
	const HAS_CONTRACT_INFO: bool;

	fn call(
		_address: &[u8; 20],
		_input: &Self::Interface,
		_env: &impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		unimplemented!("{UNIMPLEMENTED}")
	}

	fn call_with_info(
		_address: &[u8; 20],
		_input: &Self::Interface,
		_env: &impl ExtWithInfo<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		unimplemented!("{UNIMPLEMENTED}")
	}
}

/// A low level pre-compiles that does use Solidity ABI.
///
/// It is used to implement the oroginal Ethereum pre-compies which do not
/// use Solidity ABI but just encode inputs and outputs packed in memory.
///
/// Automatically implemented for all types that implement `BuiltinPrecompile`.
/// By extension also automatically implemented for all types implementing `Precompile`.
pub(crate) trait PrimitivePrecompile {
	type T: Config;
	const MATCHER: BuiltinAddressMatcher;
	const HAS_CONTRACT_INFO: bool;

	fn call(
		_address: &[u8; 20],
		_input: &[u8],
		_env: &impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Vec<u8>> {
		unimplemented!("{UNIMPLEMENTED}")
	}

	fn call_with_info(
		_address: &[u8; 20],
		_input: &[u8],
		_env: &impl ExtWithInfo<T = Self::T>,
	) -> Result<Vec<u8>, Vec<u8>> {
		unimplemented!("{UNIMPLEMENTED}")
	}
}

/// Essentially if the pre-compile in question has `HAS_CONTRACT_INFO = true`.
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Kind {
	NoContractInfo,
	WithContractInfo,
}

/// A composition of pre-compiles.
///
/// Automatically implemented for tuples of types that implement any of the
/// pre-compile traits.
pub(crate) trait Precompiles<T: Config> {
	/// Used to generate compile time error when multipe pre-compiles use the same matcher.
	const CHECK_COLLISION: ();
	/// Does any of the pre-compiles use the range reserved for external pre-compiles.
	///
	/// This is just used to generate a compile time error if `Builtin` is using the external
	/// range by accident.
	const USES_EXTERNAL_RANGE: bool;

	/// Does a pre-compile exist at `address` and if yes which kind.
	fn kind(address: &[u8; 20]) -> Option<Kind>;

	/// Try to call the pre-compile at `address`.
	///
	/// Returns `None` if no pre-compile exists at `address`.
	fn call(address: &[u8; 20], input: &[u8], env: &impl ExtWithInfo<T = T>) -> Option<ExecResult>;
}

impl<P: Precompile> BuiltinPrecompile for P {
	type T = <Self as Precompile>::T;
	type Interface = <Self as Precompile>::Interface;
	const MATCHER: BuiltinAddressMatcher = P::MATCHER.into_builtin();
	const HAS_CONTRACT_INFO: bool = P::HAS_CONTRACT_INFO;

	fn call(
		address: &[u8; 20],
		input: &Self::Interface,
		env: &impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		Self::call(address, input, env)
	}

	fn call_with_info(
		address: &[u8; 20],
		input: &Self::Interface,
		env: &impl ExtWithInfo<T = Self::T>,
	) -> Result<Vec<u8>, Revert> {
		Self::call_with_info(address, input, env)
	}
}

impl<P: BuiltinPrecompile> PrimitivePrecompile for P {
	type T = <Self as BuiltinPrecompile>::T;
	const MATCHER: BuiltinAddressMatcher = P::MATCHER;
	const HAS_CONTRACT_INFO: bool = P::HAS_CONTRACT_INFO;

	fn call(
		address: &[u8; 20],
		input: &[u8],
		env: &impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Vec<u8>> {
		let call = <Self as BuiltinPrecompile>::Interface::abi_decode(input, true)
			.map_err(|_| Panic::from(PanicKind::Generic).abi_encode())?;
		match <Self as BuiltinPrecompile>::call(address, &call, env) {
			Ok(value) => Ok(value),
			Err(err) => Err(err.abi_encode()),
		}
	}

	fn call_with_info(
		address: &[u8; 20],
		input: &[u8],
		env: &impl ExtWithInfo<T = Self::T>,
	) -> Result<Vec<u8>, Vec<u8>> {
		let call = <Self as BuiltinPrecompile>::Interface::abi_decode(input, true)
			.map_err(|_| Panic::from(PanicKind::Generic).abi_encode())?;
		match <Self as BuiltinPrecompile>::call_with_info(address, &call, env) {
			Ok(value) => Ok(value),
			Err(err) => Err(err.abi_encode()),
		}
	}
}

#[impl_trait_for_tuples::impl_for_tuples(10)]
#[tuple_types_custom_trait_bound(PrimitivePrecompile<T=T>)]
impl<T: Config> Precompiles<T> for Tuple {
	const CHECK_COLLISION: () = {
		let matchers = [for_tuples!( #( Tuple::MATCHER ),* )];
		if BuiltinAddressMatcher::has_duplicates(&matchers) {
			panic!("Precompiles with duplicate matcher detected")
		}
	};
	const USES_EXTERNAL_RANGE: bool = {
		let mut uses_external = false;
		for_tuples!(
			#(
				if Tuple::MATCHER.suffix() > u32::MAX as u64 {
					uses_external = true;
				}
			)*
		);
		uses_external
	};

	fn kind(address: &[u8; 20]) -> Option<Kind> {
		let _ = <Self as Precompiles<T>>::CHECK_COLLISION;
		for_tuples!(
			#(
				if Tuple::MATCHER.matches(address) {
					if Tuple::HAS_CONTRACT_INFO {
						return Some(Kind::WithContractInfo)
					} else {
						return Some(Kind::NoContractInfo)
					}
				}
			)*
		);
		None
	}

	fn call(address: &[u8; 20], input: &[u8], env: &impl ExtWithInfo<T = T>) -> Option<ExecResult> {
		let _ = <Self as Precompiles<T>>::CHECK_COLLISION;
		for_tuples!(
			#(
				if Tuple::MATCHER.matches(address) {
					let result = if Tuple::HAS_CONTRACT_INFO {
						Tuple::call_with_info(address, input, env)
					} else {
						Tuple::call(address, input, env)
					};
					return match result {
						Ok(data) => Some(Ok(ExecReturnValue { flags: ReturnFlags::empty(), data })),
						Err(data) => Some(Ok(ExecReturnValue { flags: ReturnFlags::REVERT, data })),
					}
				}
			)*
		);
		None
	}
}

impl<T: Config> Precompiles<T> for (Builtin<T>, <T as Config>::Precompiles) {
	const CHECK_COLLISION: () = {
		assert!(
			!<Builtin<T>>::USES_EXTERNAL_RANGE,
			"Builtin precompiles must not use addresses reserved for external precompiles"
		);
	};
	const USES_EXTERNAL_RANGE: bool = { <T as Config>::Precompiles::USES_EXTERNAL_RANGE };

	fn kind(address: &[u8; 20]) -> Option<Kind> {
		let _ = <Self as Precompiles<T>>::CHECK_COLLISION;
		<Builtin<T>>::kind(address).or_else(|| <T as Config>::Precompiles::kind(address))
	}

	fn call(address: &[u8; 20], input: &[u8], env: &impl ExtWithInfo<T = T>) -> Option<ExecResult> {
		let _ = <Self as Precompiles<T>>::CHECK_COLLISION;
		<Builtin<T>>::call(address, input, env)
			.or_else(|| <T as Config>::Precompiles::call(address, input, env))
	}
}

impl AddressMatcher {
	pub const fn base_address(&self) -> [u8; 20] {
		self.into_builtin().base_address()
	}

	pub const fn highest_address(&self) -> [u8; 20] {
		self.into_builtin().highest_address()
	}

	pub const fn matches(&self, address: &[u8; 20]) -> bool {
		self.into_builtin().matches(address)
	}

	const fn into_builtin(&self) -> BuiltinAddressMatcher {
		const fn left_shift(val: NonZero<u32>) -> NonZero<u64> {
			let shifted = (val.get() as u64) << 32;
			NonZero::new(shifted).expect(
				"Value was non zero before.
				The shift is small enough to not truncate any existing bits.
				Hence the value is still non zero; qed",
			)
		}

		match self {
			Self::Fixed(i) => BuiltinAddressMatcher::Fixed(left_shift(*i)),
			Self::Prefix(i) => BuiltinAddressMatcher::Prefix(left_shift(*i)),
		}
	}
}

impl BuiltinAddressMatcher {
	const fn suffix(&self) -> u64 {
		match self {
			Self::Fixed(i) => i.get(),
			Self::Prefix(i) => i.get(),
		}
	}

	const fn base_address(&self) -> [u8; 20] {
		let suffix = self.suffix().to_be_bytes();
		let mut address = [0u8; 20];
		let mut i = 12;
		while i < address.len() {
			address[i] = suffix[i - 12];
			i = i + 1;
		}
		address
	}

	const fn highest_address(&self) -> [u8; 20] {
		let mut address = self.base_address();
		match self {
			Self::Fixed(_) => (),
			Self::Prefix(_) => {
				address[0] = 0xFF;
				address[1] = 0xFF;
				address[2] = 0xFF;
				address[3] = 0xFF;
			},
		}
		address
	}

	const fn matches(&self, address: &[u8; 20]) -> bool {
		let base_address = self.base_address();
		let mut i = match self {
			Self::Fixed(_) => 0,
			Self::Prefix(_) => 4,
		};
		while i < base_address.len() {
			if address[i] != base_address[i] {
				return false
			}
			i = i + 1;
		}
		true
	}

	const fn has_duplicates(nums: &[Self]) -> bool {
		let len = nums.len();
		let mut i = 0;
		while i < len {
			let mut j = i + 1;
			while j < len {
				if nums[i].suffix() == nums[j].suffix() {
					return true;
				}
				j += 1;
			}
			i += 1;
		}
		false
	}
}
