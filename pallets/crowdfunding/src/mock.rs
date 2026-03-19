use std::{cell::RefCell, collections::BTreeMap};

use frame_support::{
    parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64},
    BoundedVec, PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use crate as pallet_crowdfunding;

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
        Crowdfunding: pallet_crowdfunding,
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

// ── MockNftInspect ──────────────────────────────────────────────────────

pub struct MockNftInspect;

thread_local! {
    pub static NFT_OWNERS: RefCell<BTreeMap<(u32, u32), u64>> = RefCell::new(BTreeMap::new());
}

impl MockNftInspect {
    pub fn set_owner(collection: u32, item: u32, owner: u64) {
        NFT_OWNERS.with(|m| m.borrow_mut().insert((collection, item), owner));
    }

    pub fn clear() { NFT_OWNERS.with(|m| m.borrow_mut().clear()); }
}

impl frame_support::traits::tokens::nonfungibles_v2::Inspect<u64> for MockNftInspect {
    type CollectionId = u32;
    type ItemId = u32;

    fn owner(collection: &Self::CollectionId, item: &Self::ItemId) -> Option<u64> {
        NFT_OWNERS.with(|m| m.borrow().get(&(*collection, *item)).copied())
    }
}

// ── MockLicenseVerifier ──────────────────────────────────────────────────

pub struct MockLicenseVerifier;

thread_local! {
    /// When set, `ensure_active_license` checks this map.
    /// Key: (rwa_asset_id, participation_id), Value: (authorized_account, is_active)
    pub static LICENSE_STATE: RefCell<BTreeMap<(u32, u32), (u64, bool)>> = RefCell::new(BTreeMap::new());

    /// V1 fix: asset-level active status.
    /// Key: rwa_asset_id, Value: is_asset_active.
    /// When an asset_id is NOT present, it defaults to true (active).
    pub static ASSET_ACTIVE: RefCell<BTreeMap<u32, bool>> = RefCell::new(BTreeMap::new());

    /// V2 fix: license expiry block number.
    /// Key: (rwa_asset_id, participation_id), Value: expiry block.
    pub static LICENSE_EXPIRY: RefCell<BTreeMap<(u32, u32), u64>> = RefCell::new(BTreeMap::new());
}

impl MockLicenseVerifier {
    pub fn set_license(rwa_asset_id: u32, participation_id: u32, authorized: u64, active: bool) {
        LICENSE_STATE.with(|m| {
            m.borrow_mut().insert((rwa_asset_id, participation_id), (authorized, active));
        });
    }

    pub fn set_active(rwa_asset_id: u32, participation_id: u32, active: bool) {
        LICENSE_STATE.with(|m| {
            if let Some(entry) = m.borrow_mut().get_mut(&(rwa_asset_id, participation_id)) {
                entry.1 = active;
            }
        });
    }

    /// V1: mark an asset as active or inactive at the asset level.
    pub fn set_asset_active(rwa_asset_id: u32, active: bool) {
        ASSET_ACTIVE.with(|m| {
            m.borrow_mut().insert(rwa_asset_id, active);
        });
    }

    /// V2: set the license expiry block for a specific participation.
    pub fn set_license_expiry(rwa_asset_id: u32, participation_id: u32, expiry_block: u64) {
        LICENSE_EXPIRY.with(|m| {
            m.borrow_mut().insert((rwa_asset_id, participation_id), expiry_block);
        });
    }

    pub fn clear() {
        LICENSE_STATE.with(|m| m.borrow_mut().clear());
        ASSET_ACTIVE.with(|m| m.borrow_mut().clear());
        LICENSE_EXPIRY.with(|m| m.borrow_mut().clear());
    }

    /// V1 helper: check if the asset is active (defaults to true if not
    /// explicitly set).
    fn is_asset_active(rwa_asset_id: u32) -> bool {
        ASSET_ACTIVE.with(|m| m.borrow().get(&rwa_asset_id).copied().unwrap_or(true))
    }
}

impl crate::LicenseVerifier<u64, u64> for MockLicenseVerifier {
    fn ensure_active_license(
        rwa_asset_id: u32,
        participation_id: u32,
        who: &u64,
    ) -> frame_support::dispatch::DispatchResult {
        // V1 fix: check asset-level status first
        frame_support::ensure!(
            Self::is_asset_active(rwa_asset_id),
            sp_runtime::DispatchError::Other("AssetNotActive")
        );
        LICENSE_STATE.with(|m| {
            let map = m.borrow();
            let (authorized, active) = map
                .get(&(rwa_asset_id, participation_id))
                .ok_or(sp_runtime::DispatchError::Other("LicenseNotFound"))?;
            frame_support::ensure!(
                *who == *authorized,
                sp_runtime::DispatchError::Other("NotLicenseHolder")
            );
            frame_support::ensure!(*active, sp_runtime::DispatchError::Other("LicenseExpired"));
            Ok(())
        })
    }

    fn is_license_active(rwa_asset_id: u32, participation_id: u32) -> bool {
        // V1 fix: check asset-level status first
        if !Self::is_asset_active(rwa_asset_id) {
            return false;
        }
        LICENSE_STATE.with(|m| {
            m.borrow().get(&(rwa_asset_id, participation_id)).map_or(false, |(_, active)| *active)
        })
    }

    fn license_expiry(rwa_asset_id: u32, participation_id: u32) -> Option<u64> {
        LICENSE_EXPIRY.with(|m| m.borrow().get(&(rwa_asset_id, participation_id)).copied())
    }
}

// ── Crowdfunding config ─────────────────────────────────────────────────

parameter_types! {
    pub const CrowdfundingPalletId: PalletId = PalletId(*b"py/crwdf");
    pub const ProtocolFeeAccount: u64 = 99;
}

impl pallet_crowdfunding::Config for Test {
    type AdminOrigin = EnsureRoot<u64>;
    type AssetId = u32;
    type CampaignCreationDeposit = ConstU128<100>;
    type CollectionId = u32;
    type EarlyWithdrawalPenaltyBps = ConstU16<100>;
    // 1%
    type ForceOrigin = EnsureRoot<u64>;
    type Fungibles = Assets;
    type ItemId = u32;
    type LicenseVerifier = MockLicenseVerifier;
    type MaxCampaignDuration = ConstU64<1000>;
    type MaxCampaignsPerCreator = ConstU32<5>;
    type MaxEligibilityRules = ConstU32<3>;
    type MaxInvestmentsPerInvestor = ConstU32<5>;
    type MaxMilestones = ConstU32<5>;
    type MaxNftSets = ConstU32<3>;
    type MaxNftsPerSet = ConstU32<3>;
    type MaxWhitelistSize = ConstU32<100>;
    type MilestoneApprover = EnsureRoot<u64>;
    type MinCampaignDuration = ConstU64<10>;
    type NativeCurrency = Balances;
    type NftInspect = MockNftInspect;
    type PalletId = CrowdfundingPalletId;
    type ProtocolFeeBps = ConstU16<0>;
    type ProtocolFeeRecipient = ProtocolFeeAccount;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;

pub struct ExtBuilder {
    balances: Vec<(u64, u128)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self { balances: vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000), (DAVE, 10_000)] }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(u64, u128)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        MockNftInspect::clear();
        MockLicenseVerifier::clear();
        let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        pallet_balances::GenesisConfig::<Test> { balances: self.balances }
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}

pub type CampaignConfigOf = crate::pallet::CampaignConfigOf<Test>;

pub fn default_aon_config(deadline: u64, goal: u128) -> CampaignConfigOf {
    crate::CampaignConfig {
        funding_model: crate::FundingModel::AllOrNothing { goal },
        funding_currency: crate::PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

pub fn default_kwyr_config(deadline: u64) -> CampaignConfigOf {
    crate::CampaignConfig {
        funding_model: crate::FundingModel::KeepWhatYouRaise { soft_cap: None },
        funding_currency: crate::PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

pub fn kwyr_config_with_soft_cap(deadline: u64, soft_cap: u128) -> CampaignConfigOf {
    crate::CampaignConfig {
        funding_model: crate::FundingModel::KeepWhatYouRaise { soft_cap: Some(soft_cap) },
        funding_currency: crate::PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

pub fn milestone_config(
    deadline: u64,
    goal: u128,
    milestones: Vec<crate::Milestone>,
) -> CampaignConfigOf {
    crate::CampaignConfig {
        funding_model: crate::FundingModel::MilestoneBased {
            goal,
            milestones: BoundedVec::try_from(milestones).unwrap(),
        },
        funding_currency: crate::PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

pub fn create_funded_campaign(creator: u64, config: CampaignConfigOf) -> u32 {
    use frame_support::assert_ok;
    let id = crate::pallet::NextCampaignId::<Test>::get();
    assert_ok!(Crowdfunding::create_campaign(RuntimeOrigin::signed(creator), config, None, None));
    id
}
