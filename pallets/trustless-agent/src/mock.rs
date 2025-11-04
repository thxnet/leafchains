use frame_support::{
    parameter_types,
    traits::{ConstU16, ConstU32, ConstU64},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use crate as pallet_trustless_agent;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        TrustlessAgent: pallet_trustless_agent,
    }
);

impl system::Config for Test {
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
    type MaxConsumers = frame_support::traits::ConstU32<16>;
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
    type ExistentialDeposit = frame_support::traits::ConstU128<1>;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

// ======================================================================================
// ECONOMIC CONSTANTS (Scaled for Testing)
// ======================================================================================
//
// Production Runtime vs Test Mock Comparison:
//
// ┌──────────────────────────┬─────────────────────┬─────────────┬──────────────┐
// │ Parameter                │ Production (DOLLARS)│ Test (units)│ Scale Factor
// │
// ├──────────────────────────┼─────────────────────┼─────────────┼──────────────┤
// │ DOLLARS base unit        │ 10,000,000,000      │ 1           │ 1/10^10
// │ │ AgentDeposit             │ 100 * DOLLARS       │ 1,000       │ 1/10^9
// │ │ FeedbackDeposit          │ 10 * DOLLARS        │ 100         │ 1/10^9
// │ │ ValidatorMinStake        │ 1000 * DOLLARS      │ 10,000      │ 1/10^9
// │ │ ValidationRequestDeposit │ 5 * DOLLARS         │ 50          │ 1/10^9
// │ │ DisputeDeposit           │ 20 * DOLLARS        │ 200         │ 1/10^9
// │
// └──────────────────────────┴─────────────────────┴─────────────┴──────────────┘
//
// Key Properties Maintained:
// - Proportional relationships: AgentDeposit:FeedbackDeposit = 10:1 in both
// - Economic incentives: All ratios preserved
// - Attack costs: Relative costs remain the same
//
// Test accounts are initialized with 100,000 units, sufficient for:
// - Multiple agent registrations (1,000 each)
// - Numerous feedback submissions (100 each)
// - Validator operations (10,000 stake)
// - Dispute scenarios (200 each)
//
// ======================================================================================

// Time constants (matching runtime/general/src/constants.rs)
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SECS_PER_BLOCK: u64 = MILLISECS_PER_BLOCK / 1000;
pub const MINUTES: u64 = 60 / SECS_PER_BLOCK;
pub const HOURS: u64 = MINUTES * 60;
pub const DAYS: u64 = HOURS * 24;

// Test economic constants (scaled but proportional)
pub const TEST_UNIT: u128 = 1;
pub const AGENT_DEPOSIT: u128 = 1_000 * TEST_UNIT; // 100 DOLLARS → 1,000 units
pub const FEEDBACK_DEPOSIT: u128 = 100 * TEST_UNIT; // 10 DOLLARS → 100 units
pub const VALIDATOR_MIN_STAKE: u128 = 10_000 * TEST_UNIT; // 1000 DOLLARS → 10,000 units
pub const VALIDATION_REQUEST_DEPOSIT: u128 = 50 * TEST_UNIT; // 5 DOLLARS → 50 units
pub const DISPUTE_DEPOSIT: u128 = 200 * TEST_UNIT; // 20 DOLLARS → 200 units

parameter_types! {
    // Economic parameters (using constants defined above)
    pub const AgentDeposit: u128 = AGENT_DEPOSIT;
    pub const FeedbackDeposit: u128 = FEEDBACK_DEPOSIT;
    pub const ValidatorMinStake: u128 = VALIDATOR_MIN_STAKE;
    pub const ValidationRequestDeposit: u128 = VALIDATION_REQUEST_DEPOSIT;
    pub const DisputeDeposit: u128 = DISPUTE_DEPOSIT;

    // Storage limits (same as production)
    pub const MaxUriLength: u32 = 256;
    pub const MaxTagLength: u32 = 32;
    pub const MaxTags: u32 = 10;
    pub const MaxMetadataKeyLength: u32 = 64;
    pub const MaxMetadataValueLength: u32 = 256;
    pub const MaxMetadataEntries: u32 = 20;
    pub const MaxResponsesPerFeedback: u32 = 10;

    // Time-based parameters (matching runtime constants)
    pub const ValidationDeadline: u64 = 100;
    // Escrow auto-complete duration: 7 days (7 * 14,400 blocks = 100,800 blocks)
    pub const EscrowAutoCompleteBlocks: u64 = 7 * DAYS;
    // Feedback rate limit: 7 days between feedbacks (same client to same agent)
    pub const FeedbackRateLimitBlocks: u64 = 7 * DAYS;
}

impl pallet_trustless_agent::Config for Test {
    type AgentDeposit = AgentDeposit;
    type Currency = Balances;
    type DisputeDeposit = DisputeDeposit;
    type DisputeResolverOrigin = frame_system::EnsureRoot<u64>;
    type EscrowAutoCompleteBlocks = EscrowAutoCompleteBlocks;
    type FeedbackDeposit = FeedbackDeposit;
    type FeedbackRateLimitBlocks = FeedbackRateLimitBlocks;
    type MaxMetadataEntries = MaxMetadataEntries;
    type MaxMetadataKeyLength = MaxMetadataKeyLength;
    type MaxMetadataValueLength = MaxMetadataValueLength;
    type MaxResponsesPerFeedback = MaxResponsesPerFeedback;
    type MaxTagLength = MaxTagLength;
    type MaxTags = MaxTags;
    type MaxUriLength = MaxUriLength;
    type RuntimeEvent = RuntimeEvent;
    type ValidationDeadline = ValidationDeadline;
    type ValidationRequestDeposit = ValidationRequestDeposit;
    type ValidatorManagerOrigin = frame_system::EnsureRoot<u64>;
    type ValidatorMinStake = ValidatorMinStake;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();

    // Calculate minimum required balance for comprehensive testing
    // An account should be able to:
    // - Register as agent: AGENT_DEPOSIT = 1,000
    // - Register as validator: VALIDATOR_MIN_STAKE = 10,000
    // - Submit multiple feedbacks: FEEDBACK_DEPOSIT * 10 = 1,000
    // - Create disputes: DISPUTE_DEPOSIT * 5 = 1,000
    // - Request validations: VALIDATION_REQUEST_DEPOSIT * 10 = 500
    // - Buffer for transfers and fees: 5,000
    // Total recommended: 100,000 (provides 10x buffer)
    const INITIAL_BALANCE: u128 = 100_000;

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, INITIAL_BALANCE), // AGENT_OWNER
            (2, INITIAL_BALANCE), // CLIENT_1
            (3, INITIAL_BALANCE), // CLIENT_2
            (4, INITIAL_BALANCE), // VALIDATOR_1
            (5, INITIAL_BALANCE), // VALIDATOR_2
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
