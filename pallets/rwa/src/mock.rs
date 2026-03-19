use std::{cell::RefCell, collections::BTreeSet};

use frame_support::{
    parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64},
    PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};

use crate as pallet_rwa;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Rwa: pallet_rwa,
    }
);

impl frame_system::Config for Test {
    type AccountData = pallet_balances::AccountData<u128>;
    type AccountId = u64;
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
    type SS58Prefix = ConstU16<42>;
    type SystemWeightInfo = ();
    type Version = ();
}

impl pallet_balances::Config for Test {
    type AccountStore = System;
    type Balance = u128;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

impl pallet_assets::Config for Test {
    type ApprovalDeposit = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<1>;
    type AssetDeposit = ConstU128<1>;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Balance = u128;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<u64>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<u64>;
    type Freezer = ();
    type MetadataDepositBase = ConstU128<1>;
    type MetadataDepositPerByte = ConstU128<1>;
    type RemoveItemsLimit = ConstU32<1000>;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = ConstU32<50>;
    type WeightInfo = ();
}

parameter_types! {
    pub const RwaPalletId: PalletId = PalletId(*b"py/rwaaa");
}

// ── MockLifecycleGuard ──────────────────────────────────────────────────
//
// Thread-local mock for `AssetLifecycleGuard`.  Per-test configuration
// allows selective blocking of `force_retire_asset` and
// `slash_participation` without touching the production trait impl.

pub struct MockLifecycleGuard;

thread_local! {
    static BLOCK_RETIRE: RefCell<BTreeSet<u32>> = RefCell::new(BTreeSet::new());
    static BLOCK_SLASH: RefCell<BTreeSet<(u32, u32)>> = RefCell::new(BTreeSet::new());
}

impl MockLifecycleGuard {
    /// Configure the guard to block `force_retire_asset` for `asset_id`.
    pub fn block_retire(asset_id: u32) {
        BLOCK_RETIRE.with(|s| {
            s.borrow_mut().insert(asset_id);
        });
    }

    /// Configure the guard to block `slash_participation` for the given pair.
    pub fn block_slash(asset_id: u32, participation_id: u32) {
        BLOCK_SLASH.with(|s| {
            s.borrow_mut().insert((asset_id, participation_id));
        });
    }

    /// Remove the retire block for `asset_id`.
    pub fn unblock_retire(asset_id: u32) {
        BLOCK_RETIRE.with(|s| {
            s.borrow_mut().remove(&asset_id);
        });
    }

    /// Remove the slash block for the given pair.
    pub fn unblock_slash(asset_id: u32, participation_id: u32) {
        BLOCK_SLASH.with(|s| {
            s.borrow_mut().remove(&(asset_id, participation_id));
        });
    }

    /// Reset all blocks.  Called in `ExtBuilder::build()`.
    pub fn clear() {
        BLOCK_RETIRE.with(|s| s.borrow_mut().clear());
        BLOCK_SLASH.with(|s| s.borrow_mut().clear());
    }
}

impl crate::AssetLifecycleGuard<u64> for MockLifecycleGuard {
    fn can_retire_asset(rwa_asset_id: u32) -> sp_runtime::DispatchResult {
        if BLOCK_RETIRE.with(|s| s.borrow().contains(&rwa_asset_id)) {
            Err(sp_runtime::DispatchError::Other("BlockedByLifecycleGuard"))
        } else {
            Ok(())
        }
    }

    fn can_slash_participation(
        rwa_asset_id: u32,
        participation_id: u32,
    ) -> sp_runtime::DispatchResult {
        if BLOCK_SLASH.with(|s| s.borrow().contains(&(rwa_asset_id, participation_id))) {
            Err(sp_runtime::DispatchError::Other("BlockedByLifecycleGuard"))
        } else {
            Ok(())
        }
    }
}

impl pallet_rwa::Config for Test {
    type AdminOrigin = EnsureRoot<u64>;
    type AssetId = u32;
    type AssetLifecycleGuard = MockLifecycleGuard;
    type AssetRegistrationDeposit = ConstU128<100>;
    type ForceOrigin = EnsureRoot<u64>;
    type Fungibles = Assets;
    type MaxAssetsPerOwner = ConstU32<5>;
    type MaxGroupSize = ConstU32<5>;
    type MaxMetadataLen = ConstU32<64>;
    type MaxParticipationsPerHolder = ConstU32<5>;
    type MaxPendingApprovals = ConstU32<5>;
    type MaxSlashRecipients = ConstU32<3>;
    type MaxSunsettingPerBlock = ConstU32<3>;
    /// V5: set to 1 so that zero-deposit policies are rejected.
    type MinParticipationDeposit = ConstU128<1>;
    type NativeCurrency = Balances;
    type PalletId = RwaPalletId;
    type ParticipationFilter = ();
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const EVE: u64 = 5;

pub struct ExtBuilder {
    balances: Vec<(u64, u128)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![
                (ALICE, 10_000),
                (BOB, 10_000),
                (CHARLIE, 10_000),
                (DAVE, 10_000),
                (EVE, 10_000),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(u64, u128)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        MockLifecycleGuard::clear();
        let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        let mut balances = self.balances;
        // Seed pallet account with ED so KeepAlive transfers don't fail on last exit
        let pallet_acct: u64 = RwaPalletId::get().into_account_truncating();
        balances.push((pallet_acct, 1));
        pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut t).unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn run_to_block(n: u64) {
    use frame_support::traits::Hooks;
    while System::block_number() < n {
        let next = System::block_number() + 1;
        System::set_block_number(next);
        Rwa::on_initialize(next);
    }
}

pub fn default_policy() -> crate::AssetPolicy<u128, u64, u32> {
    crate::AssetPolicy {
        deposit_currency: crate::PaymentCurrency::Native,
        entry_fee: 0,
        deposit: 50,
        max_duration: None,
        max_participants: None,
        requires_approval: false,
    }
}

pub fn approval_policy() -> crate::AssetPolicy<u128, u64, u32> {
    crate::AssetPolicy {
        deposit_currency: crate::PaymentCurrency::Native,
        entry_fee: 10,
        deposit: 50,
        max_duration: None,
        max_participants: None,
        requires_approval: true,
    }
}

pub fn timed_policy(duration: u64) -> crate::AssetPolicy<u128, u64, u32> {
    crate::AssetPolicy {
        deposit_currency: crate::PaymentCurrency::Native,
        entry_fee: 0,
        deposit: 50,
        max_duration: Some(duration),
        max_participants: None,
        requires_approval: false,
    }
}

pub fn capped_policy(max_participants: u32) -> crate::AssetPolicy<u128, u64, u32> {
    crate::AssetPolicy {
        deposit_currency: crate::PaymentCurrency::Native,
        entry_fee: 0,
        deposit: 50,
        max_duration: None,
        max_participants: Some(max_participants),
        requires_approval: false,
    }
}

pub fn register_test_asset(
    owner: u64,
    beneficiary: u64,
    policy: crate::AssetPolicy<u128, u64, u32>,
) -> u32 {
    use frame_support::assert_ok;
    let id = crate::pallet::NextRwaAssetId::<Test>::get();
    assert_ok!(Rwa::register_asset(
        RuntimeOrigin::signed(owner),
        beneficiary,
        policy,
        vec![0u8; 10],
    ));
    id
}
