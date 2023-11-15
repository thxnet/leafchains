// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test environment for Nfts pallet.

use std::sync::Arc;

use frame_support::{
    construct_runtime, parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU32, ConstU64},
};
use sp_core::H256;
use sp_keystore::{testing::KeyStore, KeystoreExt};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
    MultiSignature,
};

use super::*;
use crate as pallet_nfts;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Nfts: pallet_nfts::{Pallet, Call, Storage, Event<T>},
    }
);

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

impl frame_system::Config for Test {
    type AccountData = pallet_balances::AccountData<u64>;
    type AccountId = AccountId;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = ConstU64<250>;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type DbWeight = ();
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type MaxConsumers = ConstU32<16>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type PalletInfo = PalletInfo;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type SS58Prefix = ();
    type SystemWeightInfo = ();
    type Version = ();
}

impl pallet_balances::Config for Test {
    type AccountStore = System;
    type Balance = u64;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU64<1>;
    type MaxLocks = ();
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

parameter_types! {
    pub storage Features: PalletFeatures = PalletFeatures::all_enabled();
}

impl Config for Test {
    type ApprovalsLimit = ConstU32<10>;
    type AttributeDepositBase = ConstU64<1>;
    type CollectionDeposit = ConstU64<2>;
    type CollectionId = u32;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type Currency = Balances;
    type DepositPerByte = ConstU64<1>;
    type Features = Features;
    type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    type ItemAttributesApprovalsLimit = ConstU32<2>;
    type ItemDeposit = ConstU64<1>;
    type ItemId = u32;
    type KeyLimit = ConstU32<50>;
    type Locker = ();
    type MaxAttributesPerCall = ConstU32<2>;
    type MaxDeadlineDuration = ConstU64<10000>;
    type MaxTips = ConstU32<10>;
    type MetadataDepositBase = ConstU64<1>;
    /// Using `AccountPublic` here makes it trivial to convert to `AccountId`
    /// via `into_account()`.
    type OffchainPublic = AccountPublic;
    /// Off-chain = signature On-chain - therefore no conversion needed.
    /// It needs to be From<MultiSignature> for benchmarking.
    type OffchainSignature = Signature;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = ConstU32<50>;
    type ValueLimit = ConstU32<50>;
    type WeightInfo = ();
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(1));
    ext
}
