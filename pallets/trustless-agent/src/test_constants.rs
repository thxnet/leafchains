/// Test constants to eliminate magic numbers and improve test readability
///
/// This module provides named constants for all test parameters, making tests
/// self-documenting and easier to maintain.

// Account IDs for test scenarios
pub const AGENT_OWNER: u64 = 1;
pub const CLIENT_1: u64 = 2;
pub const CLIENT_2: u64 = 3;
pub const VALIDATOR_1: u64 = 4;
pub const VALIDATOR_2: u64 = 5;
pub const NON_PARTICIPANT: u64 = 99;

// Initial balances (aligned with mock.rs genesis)
pub const INITIAL_BALANCE: u128 = 100_000;

// Feedback score ranges (0-100)
pub const FEEDBACK_SCORE_EXCELLENT: u8 = 95;
pub const FEEDBACK_SCORE_GOOD: u8 = 85;
pub const FEEDBACK_SCORE_AVERAGE: u8 = 70;
pub const FEEDBACK_SCORE_POOR: u8 = 40;
pub const FEEDBACK_SCORE_INVALID: u8 = 101; // > 100, for testing validation

// Authorization index limits
pub const AUTH_INDEX_LIMIT_LOW: u32 = 2;
pub const AUTH_INDEX_LIMIT_MEDIUM: u32 = 10;
pub const AUTH_INDEX_LIMIT_HIGH: u32 = 100;

// Time constants (in blocks)
// Based on 6-second blocks:
// - 1 minute = 10 blocks
// - 1 hour = 600 blocks
// - 1 day = 14,400 blocks
// - 1 week = 100,800 blocks
pub const BLOCKS_PER_MINUTE: u64 = 10;
pub const BLOCKS_PER_HOUR: u64 = 600;
pub const BLOCKS_PER_DAY: u64 = 14_400;
pub const BLOCKS_PER_WEEK: u64 = 100_800;

// Authorization expiry periods
pub const AUTH_EXPIRY_SHORT: u64 = BLOCKS_PER_DAY; // 1 day
pub const AUTH_EXPIRY_MEDIUM: u64 = BLOCKS_PER_WEEK; // 1 week
pub const AUTH_EXPIRY_LONG: u64 = 4 * BLOCKS_PER_WEEK; // 4 weeks

// Escrow timeouts
pub const ESCROW_TIMEOUT_SHORT: u64 = BLOCKS_PER_DAY; // 1 day
pub const ESCROW_TIMEOUT_MEDIUM: u64 = 3 * BLOCKS_PER_DAY; // 3 days
pub const ESCROW_TIMEOUT_LONG: u64 = BLOCKS_PER_WEEK; // 1 week

// Validation deadlines
pub const VALIDATION_DEADLINE_SHORT: u64 = BLOCKS_PER_DAY; // 1 day
pub const VALIDATION_DEADLINE_STANDARD: u64 = 7 * BLOCKS_PER_DAY; // 7 days

// Helper functions for generating test data
pub fn test_uri(suffix: &str) -> Vec<u8> { format!("ipfs://QmTest{}", suffix).into_bytes() }

pub fn agent_uri(id: u64) -> Vec<u8> { format!("ipfs://QmAgent{}", id).into_bytes() }

pub fn feedback_uri(id: u64) -> Vec<u8> { format!("ipfs://QmFeedback{}", id).into_bytes() }

pub fn validation_uri(id: u64) -> Vec<u8> { format!("ipfs://QmValidation{}", id).into_bytes() }

pub fn dispute_uri(id: u64) -> Vec<u8> { format!("ipfs://QmDispute{}", id).into_bytes() }

// Common metadata for testing
pub fn test_metadata() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (b"name".to_vec(), b"TestAgent".to_vec()),
        (b"description".to_vec(), b"A test agent for unit testing".to_vec()),
    ]
}

// Common tags for testing
pub fn test_tags() -> Vec<Vec<u8>> { vec![b"helpful".to_vec(), b"responsive".to_vec()] }

// Escrow amounts (scaled for test environment)
pub const ESCROW_AMOUNT_SMALL: u128 = 100;
pub const ESCROW_AMOUNT_MEDIUM: u128 = 500;
pub const ESCROW_AMOUNT_LARGE: u128 = 1000;

// Validator stake amounts (scaled for test environment)
pub const VALIDATOR_STAKE_MIN: u128 = 10_000; // Must match ValidatorMinStake in mock
pub const VALIDATOR_STAKE_MEDIUM: u128 = 20_000;
pub const VALIDATOR_STAKE_HIGH: u128 = 50_000;

// Validation scores (0-100)
pub const VALIDATION_SCORE_EXCELLENT: u8 = 95;
pub const VALIDATION_SCORE_GOOD: u8 = 85;
pub const VALIDATION_SCORE_AVERAGE: u8 = 70;
pub const VALIDATION_SCORE_POOR: u8 = 40;

// Helper to calculate expected reputation scores
// Reputation is scaled to 0-10000 (representing 0.00% to 100.00%)
pub fn to_reputation_scale(score: u8) -> u32 { (score as u32) * 100 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_constants_are_correct() {
        // Verify time constant calculations
        assert_eq!(BLOCKS_PER_MINUTE, 10);
        assert_eq!(BLOCKS_PER_HOUR, 600);
        assert_eq!(BLOCKS_PER_DAY, 14_400);
        assert_eq!(BLOCKS_PER_WEEK, 100_800);
    }

    #[test]
    fn reputation_scale_is_correct() {
        assert_eq!(to_reputation_scale(0), 0);
        assert_eq!(to_reputation_scale(50), 5000);
        assert_eq!(to_reputation_scale(100), 10000);
    }

    #[test]
    fn test_uri_formats_correctly() {
        assert_eq!(test_uri("123"), b"ipfs://QmTest123");
        assert_eq!(feedback_uri(42), b"ipfs://QmFeedback42");
    }
}
