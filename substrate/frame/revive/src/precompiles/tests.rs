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

use super::*;
use crate::tests::Test;
use core::num::NonZero;
use sp_core::hex2array as hex;

#[test]
fn matching_works() {
	struct Matcher1;
	struct Matcher2;

	impl PrimitivePrecompile for Matcher1 {
		type T = Test;
		const MATCHER: BuiltinAddressMatcher =
			BuiltinAddressMatcher::Fixed(NonZero::new(0x42).unwrap());
		const HAS_CONTRACT_INFO: bool = true;

		fn call(
			address: &[u8; 20],
			_input: &[u8],
			_env: &impl Ext<T = Self::T>,
		) -> Result<Vec<u8>, Vec<u8>> {
			Ok(address.to_vec())
		}
	}

	impl PrimitivePrecompile for Matcher2 {
		type T = Test;
		const MATCHER: BuiltinAddressMatcher =
			BuiltinAddressMatcher::Prefix(NonZero::new(0x88).unwrap());
		const HAS_CONTRACT_INFO: bool = false;

		fn call(
			address: &[u8; 20],
			_input: &[u8],
			_env: &impl Ext<T = Self::T>,
		) -> Result<Vec<u8>, Vec<u8>> {
			Ok(address.to_vec())
		}
	}

	type Col = (Matcher1, Matcher2);

	assert_eq!(
		<Matcher1 as PrimitivePrecompile>::MATCHER.base_address(),
		hex!("0000000000000000000000000000000000000042")
	);
	assert_eq!(
		<Matcher1 as PrimitivePrecompile>::MATCHER.base_address(),
		<Matcher1 as PrimitivePrecompile>::MATCHER.highest_address()
	);

	assert_eq!(
		<Matcher2 as PrimitivePrecompile>::MATCHER.base_address(),
		hex!("0000000000000000000000000000000000000088")
	);
	assert_eq!(
		<Matcher2 as PrimitivePrecompile>::MATCHER.highest_address(),
		hex!("FFFFFFFF00000000000000000000000000000088")
	);

	assert_eq!(Col::kind(&hex!("1000000000000000000000000000000000000043")), None,);
	assert_eq!(
		Col::kind(&hex!("0000000000000000000000000000000000000042")),
		Some(Kind::WithContractInfo)
	);
	assert_eq!(Col::kind(&hex!("1000000000000000000000000000000000000042")), None,);
	assert_eq!(
		Col::kind(&hex!("0000000000000000000000000000000000000088")),
		Some(Kind::NoContractInfo)
	);
	assert_eq!(
		Col::kind(&hex!("2200000000000000000000000000000000000088")),
		Some(Kind::NoContractInfo)
	);
	assert_eq!(
		Col::kind(&hex!("0010000000000000000000000000000000000088")),
		Some(Kind::NoContractInfo)
	);
	assert_eq!(Col::kind(&hex!("0000000010000000000000000000000000000088")), None);
}

#[test]
fn builtin_matching_works() {
	let _ = <All<Test>>::CHECK_COLLISION;

	assert_eq!(
		<Builtin<Test>>::kind(&hex!("0000000000000000000000000000000000000001")),
		Some(Kind::NoContractInfo)
	);
	assert_eq!(
		<Builtin<Test>>::kind(&hex!("0000000000000000000000000000000000001000")),
		Some(Kind::NoContractInfo)
	);
	assert_eq!(<Builtin<Test>>::kind(&hex!("7000000000000000000000000000000000000001")), None,);
	assert_eq!(<Builtin<Test>>::kind(&hex!("7000000000000000000000000000000000001000")), None,);
}

#[test]
fn public_matching_works() {
	let matcher_fixed = AddressMatcher::Fixed(NonZero::new(0x42).unwrap());
	let matcher_prefix = AddressMatcher::Prefix(NonZero::new(0x8).unwrap());

	assert_eq!(matcher_fixed.base_address(), hex!("0000000000000000000000000000004200000000"));
	assert_eq!(matcher_fixed.base_address(), matcher_fixed.highest_address());

	assert_eq!(matcher_prefix.base_address(), hex!("0000000000000000000000000000000800000000"));
	assert_eq!(matcher_prefix.highest_address(), hex!("FFFFFFFF00000000000000000000000800000000"));
}
