#![cfg_attr(not(feature = "std"), no_std)]

//! # Trustless Agent Pallet
//!
//! A Substrate pallet implementing decentralized agent identity, reputation,
//! and validation systems inspired by EIP-8004.
//!
//! ## Overview
//!
//! This pallet provides three core registries:
//! - **Identity Registry**: Agent registration and identity management
//! - **Reputation Registry**: Feedback authorization, submission, and
//!   management
//! - **Validation Registry**: Validator registration and validation workflows
//!
//! ## Interface
//!
//! ### Identity Registry
//! - `register_agent`: Register a new agent identity
//! - `update_metadata`: Update agent metadata
//! - `transfer_agent`: Transfer agent ownership
//!
//! ### Reputation Registry
//! - `give_feedback`: Submit feedback with pre-authorization
//! - `revoke_feedback`: Revoke submitted feedback
//! - `append_response`: Agent response to feedback
//!
//! ### Validation Registry
//! - `register_validator`: Register as a validator with stake
//! - `unregister_validator`: Unregister and unlock stake
//! - `request_validation`: Request validation for an agent
//! - `submit_validation`: Submit validation results

pub use pallet::*;

pub mod migrations;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Agent ID type
    pub type AgentId = u64;
    /// Feedback ID type
    pub type FeedbackId = u64;
    /// Validation Request ID type
    pub type RequestId = u64;
    /// Escrow ID type
    pub type EscrowId = u64;

    /// Agent information
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct AgentInfo<T: Config> {
        /// Agent owner
        pub owner: T::AccountId,
        /// Registration file URI (IPFS hash or HTTP URL)
        pub registration_uri: BoundedVec<u8, T::MaxUriLength>,
        /// Creation timestamp
        pub created_at: T::BlockNumber,
    }

    /// Agent metadata entry
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct MetadataEntry<T: Config> {
        pub key: BoundedVec<u8, T::MaxMetadataKeyLength>,
        pub value: BoundedVec<u8, T::MaxMetadataValueLength>,
    }

    /// Feedback information
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct FeedbackInfo<T: Config> {
        /// Agent being reviewed
        pub agent_id: AgentId,
        /// Client providing feedback
        pub client: T::AccountId,
        /// Score (0-100)
        pub score: u8,
        /// Tags for categorization
        pub tags: BoundedVec<BoundedVec<u8, T::MaxTagLength>, T::MaxTags>,
        /// URI pointing to detailed feedback data
        pub file_uri: BoundedVec<u8, T::MaxUriLength>,
        /// Hash of detailed feedback content
        pub content_hash: T::Hash,
        /// Submission timestamp
        pub created_at: T::BlockNumber,
        /// Whether this feedback has been revoked
        pub revoked: bool,
    }

    /// Feedback authorization (EIP-8004 compliant)
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct FeedbackAuthorization<T: Config> {
        /// Agent granting authorization
        pub agent_id: AgentId,
        /// Client being authorized to provide feedback
        pub client: T::AccountId,
        /// Maximum number of feedbacks allowed under this authorization
        pub index_limit: u32,
        /// Block number after which authorization expires
        pub expiry: T::BlockNumber,
        /// Timestamp when authorization was created
        pub created_at: T::BlockNumber,
        /// Whether this authorization has been revoked
        pub revoked: bool,
    }

    /// Response to feedback
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct FeedbackResponse<T: Config> {
        /// Response URI
        pub response_uri: BoundedVec<u8, T::MaxUriLength>,
        /// Hash of response content
        pub content_hash: T::Hash,
        /// Response timestamp
        pub created_at: T::BlockNumber,
    }

    /// Validator information
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ValidatorInfo<T: Config> {
        /// Validator account
        pub account: T::AccountId,
        /// Staked amount
        pub stake: BalanceOf<T>,
        /// Registration timestamp
        pub registered_at: T::BlockNumber,
        /// Whether the validator is active
        pub active: bool,
    }

    /// Validation request
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ValidationRequest<T: Config> {
        /// Agent being validated
        pub agent_id: AgentId,
        /// Account that requested validation
        pub requester: T::AccountId,
        /// Reward for validators
        pub reward: BalanceOf<T>,
        /// Request timestamp
        pub created_at: T::BlockNumber,
        /// Deadline for validation
        pub deadline: T::BlockNumber,
        /// Whether the request has been completed
        pub completed: bool,
    }

    /// Validation response
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ValidationResponse<T: Config> {
        /// Request ID
        pub request_id: RequestId,
        /// Validator account
        pub validator: T::AccountId,
        /// Validation score (0-100)
        pub score: u8,
        /// Evidence URI
        pub evidence_uri: BoundedVec<u8, T::MaxUriLength>,
        /// Hash of evidence content
        pub content_hash: T::Hash,
        /// Tags for categorization
        pub tags: BoundedVec<BoundedVec<u8, T::MaxTagLength>, T::MaxTags>,
        /// Response timestamp
        pub created_at: T::BlockNumber,
    }

    /// Escrow status
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum EscrowStatus {
        /// Escrow is active and waiting for completion
        Active,
        /// Escrow has auto-completed and agent can claim
        AutoCompleted,
        /// Client has disputed the escrow
        Disputed,
        /// Dispute has been resolved
        Resolved,
    }

    /// Escrow information
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct EscrowInfo<T: Config> {
        /// Client who created the escrow
        pub client: T::AccountId,
        /// Agent the escrow is for
        pub agent_id: AgentId,
        /// Amount locked in the escrow
        pub amount: BalanceOf<T>,
        /// Block number after which the escrow auto-completes
        pub auto_complete_at: T::BlockNumber,
        /// Block number after which the client can cancel (if not
        /// auto-completed)
        pub timeout: T::BlockNumber,
        /// Current status of the escrow
        pub status: EscrowStatus,
        /// Creation timestamp
        pub created_at: T::BlockNumber,
    }

    /// Agent reputation score
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct ReputationScore {
        /// Average feedback score (0-10000, representing 0.00-100.00)
        pub feedback_score: u32,
        /// Number of feedback entries
        pub feedback_count: u32,
        /// Average validation score (0-10000, representing 0.00-100.00)
        pub validation_score: u32,
        /// Number of validation entries
        pub validation_count: u32,
        /// Overall weighted score (0-10000, representing 0.00-100.00)
        pub overall_score: u32,
        /// Last update block number
        pub last_updated: u32,
    }

    /// Dispute ID type
    pub type DisputeId = u64;

    /// Dispute status
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum DisputeStatus {
        /// Dispute is open and awaiting resolution
        Open,
        /// Dispute was resolved in favor of the disputer
        ResolvedForDisputer,
        /// Dispute was resolved in favor of the disputee
        ResolvedAgainstDisputer,
        /// Dispute was dismissed
        Dismissed,
    }

    /// Dispute information
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DisputeInfo<T: Config> {
        /// Feedback being disputed
        pub feedback_id: FeedbackId,
        /// Account that created the dispute (usually the agent owner)
        pub disputer: T::AccountId,
        /// Reason URI
        pub reason_uri: BoundedVec<u8, T::MaxUriLength>,
        /// Hash of reason content
        pub content_hash: T::Hash,
        /// Dispute status
        pub status: DisputeStatus,
        /// Creation timestamp
        pub created_at: T::BlockNumber,
        /// Resolution timestamp (if resolved)
        pub resolved_at: Option<T::BlockNumber>,
    }

    /// Weight information for extrinsics in this pallet.
    pub trait WeightInfo {
        fn register_agent() -> Weight;
        fn update_metadata() -> Weight;
        fn transfer_agent() -> Weight;
        fn give_feedback() -> Weight;
        fn revoke_feedback() -> Weight;
        fn append_response() -> Weight;
        fn register_validator() -> Weight;
        fn unregister_validator() -> Weight;
        fn request_validation() -> Weight;
        fn submit_validation() -> Weight;
        fn create_escrow() -> Weight;
        fn claim_escrow() -> Weight;
        fn cancel_escrow() -> Weight;
        fn cancel_validation_request() -> Weight;
        fn dispute_feedback() -> Weight;
        fn resolve_dispute() -> Weight;
        fn add_validator_to_whitelist() -> Weight;
        fn remove_validator_from_whitelist() -> Weight;
    }

    /// Default weight implementation (for testing)
    pub struct SubstrateWeight<T>(sp_std::marker::PhantomData<T>);
    impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
        fn register_agent() -> Weight { Weight::from_parts(10_000, 0) }

        fn update_metadata() -> Weight { Weight::from_parts(10_000, 0) }

        fn transfer_agent() -> Weight { Weight::from_parts(10_000, 0) }

        fn give_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn revoke_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn append_response() -> Weight { Weight::from_parts(10_000, 0) }

        fn register_validator() -> Weight { Weight::from_parts(10_000, 0) }

        fn unregister_validator() -> Weight { Weight::from_parts(10_000, 0) }

        fn request_validation() -> Weight { Weight::from_parts(10_000, 0) }

        fn submit_validation() -> Weight { Weight::from_parts(10_000, 0) }

        fn create_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn claim_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn cancel_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn cancel_validation_request() -> Weight { Weight::from_parts(10_000, 0) }

        fn dispute_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn resolve_dispute() -> Weight { Weight::from_parts(10_000, 0) }

        fn add_validator_to_whitelist() -> Weight { Weight::from_parts(10_000, 0) }

        fn remove_validator_from_whitelist() -> Weight { Weight::from_parts(10_000, 0) }
    }

    /// Implementation for unit tests
    impl WeightInfo for () {
        fn register_agent() -> Weight { Weight::from_parts(10_000, 0) }

        fn update_metadata() -> Weight { Weight::from_parts(10_000, 0) }

        fn transfer_agent() -> Weight { Weight::from_parts(10_000, 0) }

        fn give_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn revoke_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn append_response() -> Weight { Weight::from_parts(10_000, 0) }

        fn register_validator() -> Weight { Weight::from_parts(10_000, 0) }

        fn unregister_validator() -> Weight { Weight::from_parts(10_000, 0) }

        fn request_validation() -> Weight { Weight::from_parts(10_000, 0) }

        fn submit_validation() -> Weight { Weight::from_parts(10_000, 0) }

        fn create_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn claim_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn cancel_escrow() -> Weight { Weight::from_parts(10_000, 0) }

        fn cancel_validation_request() -> Weight { Weight::from_parts(10_000, 0) }

        fn dispute_feedback() -> Weight { Weight::from_parts(10_000, 0) }

        fn resolve_dispute() -> Weight { Weight::from_parts(10_000, 0) }

        fn add_validator_to_whitelist() -> Weight { Weight::from_parts(10_000, 0) }

        fn remove_validator_from_whitelist() -> Weight { Weight::from_parts(10_000, 0) }
    }

    #[pallet::pallet]
    #[pallet::storage_version(crate::migrations::STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency mechanism for handling deposits and stakes.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Agent registration deposit
        #[pallet::constant]
        type AgentDeposit: Get<BalanceOf<Self>>;

        /// Feedback submission deposit
        #[pallet::constant]
        type FeedbackDeposit: Get<BalanceOf<Self>>;

        /// Validator minimum stake
        #[pallet::constant]
        type ValidatorMinStake: Get<BalanceOf<Self>>;

        /// Maximum length of URI strings
        #[pallet::constant]
        type MaxUriLength: Get<u32>;

        /// Maximum length of tag strings
        #[pallet::constant]
        type MaxTagLength: Get<u32>;

        /// Maximum number of tags
        #[pallet::constant]
        type MaxTags: Get<u32>;

        /// Maximum length of metadata keys
        #[pallet::constant]
        type MaxMetadataKeyLength: Get<u32>;

        /// Maximum length of metadata values
        #[pallet::constant]
        type MaxMetadataValueLength: Get<u32>;

        /// Maximum number of metadata entries per agent
        #[pallet::constant]
        type MaxMetadataEntries: Get<u32>;

        /// Maximum number of responses per feedback
        #[pallet::constant]
        type MaxResponsesPerFeedback: Get<u32>;

        /// Validation request deposit
        #[pallet::constant]
        type ValidationRequestDeposit: Get<BalanceOf<Self>>;

        /// Dispute deposit
        #[pallet::constant]
        type DisputeDeposit: Get<BalanceOf<Self>>;

        /// Validation deadline (number of blocks)
        #[pallet::constant]
        type ValidationDeadline: Get<Self::BlockNumber>;

        /// Escrow auto-complete duration (number of blocks, e.g., 7 days)
        #[pallet::constant]
        type EscrowAutoCompleteBlocks: Get<Self::BlockNumber>;

        /// Feedback rate limit duration (number of blocks, e.g., 7 days)
        #[pallet::constant]
        type FeedbackRateLimitBlocks: Get<Self::BlockNumber>;

        /// Origin that can resolve disputes (e.g., council or root)
        type DisputeResolverOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can manage validator whitelist
        type ValidatorManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    // ===== Storage =====

    /// Storage for escrow information
    #[pallet::storage]
    #[pallet::getter(fn escrows)]
    pub type Escrows<T: Config> = StorageMap<_, Blake2_128Concat, EscrowId, EscrowInfo<T>>;

    /// Next escrow ID
    #[pallet::storage]
    #[pallet::getter(fn next_escrow_id)]
    pub type NextEscrowId<T: Config> = StorageValue<_, EscrowId, ValueQuery>;

    // --- Identity Registry ---

    /// Storage for agent information
    #[pallet::storage]
    #[pallet::getter(fn agents)]
    pub type Agents<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, AgentInfo<T>>;

    /// Agent metadata
    #[pallet::storage]
    #[pallet::getter(fn agent_metadata)]
    pub type AgentMetadata<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxMetadataKeyLength>,
        BoundedVec<u8, T::MaxMetadataValueLength>,
    >;

    /// Agents owned by an account
    #[pallet::storage]
    #[pallet::getter(fn agent_owner)]
    pub type AgentsByOwner<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<AgentId, ConstU32<1000>>>;

    /// Next agent ID
    #[pallet::storage]
    #[pallet::getter(fn next_agent_id)]
    pub type NextAgentId<T: Config> = StorageValue<_, AgentId, ValueQuery>;

    // --- Reputation Registry ---

    /// Storage for feedback information
    #[pallet::storage]
    #[pallet::getter(fn feedbacks)]
    pub type Feedbacks<T: Config> = StorageMap<_, Blake2_128Concat, FeedbackId, FeedbackInfo<T>>;

    /// Feedback IDs for a specific agent
    #[pallet::storage]
    #[pallet::getter(fn agent_feedbacks)]
    pub type AgentFeedbacks<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, BoundedVec<FeedbackId, ConstU32<10000>>>;

    /// Feedback IDs provided by a client
    #[pallet::storage]
    #[pallet::getter(fn client_feedbacks)]
    pub type ClientFeedbacks<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<FeedbackId, ConstU32<1000>>>;

    /// Responses to feedback
    #[pallet::storage]
    #[pallet::getter(fn feedback_responses)]
    pub type FeedbackResponses<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        FeedbackId,
        BoundedVec<FeedbackResponse<T>, T::MaxResponsesPerFeedback>,
    >;

    /// Next feedback ID
    #[pallet::storage]
    #[pallet::getter(fn next_feedback_id)]
    pub type NextFeedbackId<T: Config> = StorageValue<_, FeedbackId, ValueQuery>;

    /// Feedback authorization ID type
    pub type AuthorizationId = u64;

    /// Storage for feedback authorizations
    #[pallet::storage]
    #[pallet::getter(fn feedback_authorizations)]
    pub type FeedbackAuthorizations<T: Config> =
        StorageMap<_, Blake2_128Concat, AuthorizationId, FeedbackAuthorization<T>>;

    /// Feedback authorization lookup by (agent_id, client)
    #[pallet::storage]
    #[pallet::getter(fn agent_client_authorizations)]
    pub type AgentClientAuthorizations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        T::AccountId,
        AuthorizationId,
    >;

    /// Feedback indices: tracks number of feedbacks per (agent_id, client)
    #[pallet::storage]
    #[pallet::getter(fn feedback_indices)]
    pub type FeedbackIndices<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        T::AccountId,
        u32,
        ValueQuery,
    >;

    /// Last feedback timestamp per (agent_id, client) for rate limiting
    #[pallet::storage]
    #[pallet::getter(fn last_feedback_timestamp)]
    pub type LastFeedbackTimestamp<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        T::AccountId,
        T::BlockNumber,
    >;

    /// Next authorization ID
    #[pallet::storage]
    #[pallet::getter(fn next_authorization_id)]
    pub type NextAuthorizationId<T: Config> = StorageValue<_, AuthorizationId, ValueQuery>;

    // --- Validation Registry ---

    /// Storage for validator information
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorInfo<T>>;

    /// Storage for validation requests
    #[pallet::storage]
    #[pallet::getter(fn validation_requests)]
    pub type ValidationRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, RequestId, ValidationRequest<T>>;

    /// Storage for validation responses
    #[pallet::storage]
    #[pallet::getter(fn validation_responses)]
    pub type ValidationResponses<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        RequestId,
        Blake2_128Concat,
        T::AccountId,
        ValidationResponse<T>,
    >;

    /// Validation request IDs for a specific agent
    #[pallet::storage]
    #[pallet::getter(fn agent_validations)]
    pub type AgentValidations<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, BoundedVec<RequestId, ConstU32<100>>>;

    /// Next validation request ID
    #[pallet::storage]
    #[pallet::getter(fn next_request_id)]
    pub type NextRequestId<T: Config> = StorageValue<_, RequestId, ValueQuery>;

    // --- Reputation System ---

    /// Agent reputation scores
    #[pallet::storage]
    #[pallet::getter(fn agent_reputation)]
    pub type AgentReputations<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, ReputationScore>;

    // --- Dispute System ---

    /// Storage for dispute information
    #[pallet::storage]
    #[pallet::getter(fn disputes)]
    pub type Disputes<T: Config> = StorageMap<_, Blake2_128Concat, DisputeId, DisputeInfo<T>>;

    /// Dispute IDs for a specific feedback
    #[pallet::storage]
    #[pallet::getter(fn feedback_disputes)]
    pub type FeedbackDisputes<T: Config> =
        StorageMap<_, Blake2_128Concat, FeedbackId, BoundedVec<DisputeId, ConstU32<10>>>;

    /// Next dispute ID
    #[pallet::storage]
    #[pallet::getter(fn next_dispute_id)]
    pub type NextDisputeId<T: Config> = StorageValue<_, DisputeId, ValueQuery>;

    // --- Validator Management ---

    /// Validator whitelist (optional, if None then any staker can be validator)
    #[pallet::storage]
    #[pallet::getter(fn validator_whitelist)]
    pub type ValidatorWhitelist<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    // ===== Events =====

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // --- Identity Registry Events ---
        /// An agent was registered
        AgentRegistered { agent_id: AgentId, owner: T::AccountId, registration_uri: Vec<u8> },
        /// Agent metadata was updated
        AgentMetadataUpdated { agent_id: AgentId, key: Vec<u8> },
        /// Agent ownership was transferred
        AgentTransferred { agent_id: AgentId, old_owner: T::AccountId, new_owner: T::AccountId },

        // --- Reputation Registry Events ---
        /// Feedback authorization was granted
        FeedbackAuthorizationGranted {
            authorization_id: AuthorizationId,
            agent_id: AgentId,
            client: T::AccountId,
            index_limit: u32,
            expiry: T::BlockNumber,
        },
        /// Feedback authorization was revoked
        FeedbackAuthorizationRevoked { authorization_id: AuthorizationId },
        /// Feedback was submitted
        FeedbackGiven {
            feedback_id: FeedbackId,
            agent_id: AgentId,
            client: T::AccountId,
            score: u8,
        },
        /// Feedback was revoked
        FeedbackRevoked { feedback_id: FeedbackId },
        /// Response was appended to feedback
        ResponseAppended { feedback_id: FeedbackId, responder: T::AccountId },

        // --- Validation Registry Events ---
        /// A validator was registered
        ValidatorRegistered { validator: T::AccountId, stake: BalanceOf<T> },
        /// A validator was unregistered
        ValidatorUnregistered { validator: T::AccountId },
        /// Validation was requested
        ValidationRequested { request_id: RequestId, agent_id: AgentId, requester: T::AccountId },
        /// Validation was submitted
        ValidationSubmitted { request_id: RequestId, validator: T::AccountId, score: u8 },

        // --- Escrow Events ---
        /// An escrow was created
        EscrowCreated {
            escrow_id: EscrowId,
            client: T::AccountId,
            agent_id: AgentId,
            amount: BalanceOf<T>,
            timeout: T::BlockNumber,
        },
        /// An escrow was auto-completed
        EscrowAutoCompleted { escrow_id: EscrowId },
        /// An escrow was disputed
        EscrowDisputed { escrow_id: EscrowId, disputer: T::AccountId },
        /// An escrow dispute was resolved
        EscrowDisputeResolved { escrow_id: EscrowId, status: EscrowStatus },
        /// An escrow was claimed by an agent
        EscrowClaimed { escrow_id: EscrowId, agent_id: AgentId },
        /// An escrow was cancelled by the client
        EscrowCancelled { escrow_id: EscrowId, client: T::AccountId },

        // --- Reputation System Events ---
        /// Agent reputation was updated
        ReputationUpdated { agent_id: AgentId, overall_score: u32 },

        // --- Dispute Events ---
        /// A dispute was created
        DisputeCreated { dispute_id: DisputeId, feedback_id: FeedbackId, disputer: T::AccountId },
        /// A dispute was resolved
        DisputeResolved { dispute_id: DisputeId, status: DisputeStatus },

        // --- Validator Management Events ---
        /// Validation request was cancelled
        ValidationRequestCancelled { request_id: RequestId, requester: T::AccountId },
        /// Validator was added to whitelist
        ValidatorWhitelisted { validator: T::AccountId },
        /// Validator was removed from whitelist
        ValidatorRemovedFromWhitelist { validator: T::AccountId },
        /// Validator was slashed for inactivity
        ValidatorSlashed { validator: T::AccountId, amount: BalanceOf<T> },
    }

    // ===== Errors =====

    #[pallet::error]
    pub enum Error<T> {
        // --- Identity Registry Errors ---
        /// Agent does not exist
        AgentNotFound,
        /// Not the agent owner
        NotAgentOwner,
        /// Too many agents owned by this account
        TooManyAgents,
        /// Too many metadata entries
        TooManyMetadataEntries,
        /// URI too long
        UriTooLong,
        /// Metadata key too long
        MetadataKeyTooLong,
        /// Metadata value too long
        MetadataValueTooLong,

        // --- Reputation Registry Errors ---
        /// Feedback does not exist
        FeedbackNotFound,
        /// Not the feedback client
        NotFeedbackClient,
        /// Feedback already revoked
        FeedbackAlreadyRevoked,
        /// Invalid score (must be 0-100)
        InvalidScore,
        /// Too many tags
        TooManyTags,
        /// Tag too long
        TagTooLong,
        /// Too many feedbacks
        TooManyFeedbacks,
        /// Too many responses
        TooManyResponses,
        /// Invalid signature
        InvalidSignature,
        /// Authorization not found
        AuthorizationNotFound,
        /// Authorization has expired
        AuthorizationExpired,
        /// Authorization has been revoked
        AuthorizationRevoked,
        /// Authorization index limit exceeded
        AuthorizationIndexLimitExceeded,
        /// No valid authorization found
        NoValidAuthorization,
        /// Feedback rate limit exceeded
        FeedbackRateLimitExceeded,
        /// Authorization already exists for this agent-client pair
        AuthorizationAlreadyExists,

        // --- Validation Registry Errors ---
        /// Validator already registered
        ValidatorAlreadyRegistered,
        /// Validator not found
        ValidatorNotFound,
        /// Insufficient stake
        InsufficientStake,
        /// Validation request not found
        ValidationRequestNotFound,
        /// Not the requester
        NotRequester,
        /// Validator not active
        ValidatorNotActive,
        /// Validation already submitted by this validator
        ValidationAlreadySubmitted,
        /// Too many validation requests
        TooManyValidationRequests,

        // --- Escrow Errors ---
        /// Escrow does not exist
        EscrowNotFound,
        /// Not the escrow client
        NotEscrowClient,
        /// Escrow has not yet timed out
        EscrowNotTimedOut,
        /// Escrow has already been completed or cancelled
        EscrowAlreadyClosed,
        /// Invalid signature for claiming escrow
        InvalidEscrowSignature,
        /// Escrow has not auto-completed yet
        EscrowNotAutoCompleted,
        /// Escrow is disputed and cannot be claimed
        EscrowDisputed,
        /// Cannot dispute escrow in current status
        CannotDisputeEscrow,
        /// Escrow dispute already resolved
        EscrowDisputeAlreadyResolved,
        /// Timeout must be after auto-complete time
        InvalidTimeoutPeriod,
        /// Cannot cancel escrow that has auto-completed
        CannotCancelAutoCompleted,

        // --- Dispute Errors ---
        /// Dispute does not exist
        DisputeNotFound,
        /// Not authorized to resolve dispute
        NotDisputeResolver,
        /// Dispute already resolved
        DisputeAlreadyResolved,
        /// Too many disputes for this feedback
        TooManyDisputes,
        /// Cannot dispute revoked feedback
        CannotDisputeRevokedFeedback,

        // --- Validation Request Errors ---
        /// Validation request already completed
        ValidationRequestCompleted,
        /// Validation deadline has passed
        ValidationDeadlinePassed,
        /// Validation deadline has not passed yet
        ValidationDeadlineNotPassed,

        // --- Validator Whitelist Errors ---
        /// Validator not whitelisted
        ValidatorNotWhitelisted,
        /// Validator already whitelisted
        ValidatorAlreadyWhitelisted,

        // --- General Errors ---
        /// Arithmetic overflow
        Overflow,
    }

    // ===== Extrinsics =====

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ===== Identity Registry =====

        /// Register a new agent
        ///
        /// # Parameters
        /// - `registration_uri`: URI pointing to registration file (IPFS hash
        ///   or HTTP URL)
        /// - `metadata`: Optional initial metadata entries
        ///
        /// # Errors
        /// - `UriTooLong`: URI exceeds maximum length
        /// - `TooManyMetadataEntries`: Too many metadata entries provided
        /// - `TooManyAgents`: Owner has too many agents
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_agent())]
        pub fn register_agent(
            origin: OriginFor<T>,
            registration_uri: Vec<u8>,
            metadata: Vec<(Vec<u8>, Vec<u8>)>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate URI length
            let bounded_uri: BoundedVec<u8, T::MaxUriLength> =
                registration_uri.clone().try_into().map_err(|_| Error::<T>::UriTooLong)?;

            // Validate metadata count
            ensure!(
                metadata.len() <= T::MaxMetadataEntries::get() as usize,
                Error::<T>::TooManyMetadataEntries
            );

            // Reserve deposit
            T::Currency::reserve(&who, T::AgentDeposit::get())?;

            // Get next agent ID
            let agent_id = NextAgentId::<T>::get();
            let next_id = agent_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Create agent info
            let agent_info = AgentInfo {
                owner: who.clone(),
                registration_uri: bounded_uri,
                created_at: frame_system::Pallet::<T>::block_number(),
            };

            // Store agent
            Agents::<T>::insert(agent_id, agent_info);
            NextAgentId::<T>::put(next_id);

            // Store metadata
            for (key, value) in metadata {
                let bounded_key: BoundedVec<u8, T::MaxMetadataKeyLength> =
                    key.clone().try_into().map_err(|_| Error::<T>::MetadataKeyTooLong)?;
                let bounded_value: BoundedVec<u8, T::MaxMetadataValueLength> =
                    value.try_into().map_err(|_| Error::<T>::MetadataValueTooLong)?;

                AgentMetadata::<T>::insert(agent_id, bounded_key, bounded_value);
            }

            // Update owner's agent list
            AgentsByOwner::<T>::try_mutate(&who, |agents| -> DispatchResult {
                let mut agent_list = agents.take().unwrap_or_default();
                agent_list.try_push(agent_id).map_err(|_| Error::<T>::TooManyAgents)?;
                *agents = Some(agent_list);
                Ok(())
            })?;

            Self::deposit_event(Event::AgentRegistered { agent_id, owner: who, registration_uri });

            Ok(())
        }

        /// Update agent metadata
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent
        /// - `key`: Metadata key
        /// - `value`: Metadata value (None to remove)
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_metadata())]
        pub fn update_metadata(
            origin: OriginFor<T>,
            agent_id: AgentId,
            key: Vec<u8>,
            value: Option<Vec<u8>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists and caller is owner
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            let bounded_key: BoundedVec<u8, T::MaxMetadataKeyLength> =
                key.clone().try_into().map_err(|_| Error::<T>::MetadataKeyTooLong)?;

            if let Some(val) = value {
                let bounded_value: BoundedVec<u8, T::MaxMetadataValueLength> =
                    val.try_into().map_err(|_| Error::<T>::MetadataValueTooLong)?;
                AgentMetadata::<T>::insert(agent_id, &bounded_key, bounded_value);
            } else {
                AgentMetadata::<T>::remove(agent_id, &bounded_key);
            }

            Self::deposit_event(Event::AgentMetadataUpdated { agent_id, key });

            Ok(())
        }

        /// Transfer agent ownership
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent
        /// - `new_owner`: New owner account
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::transfer_agent())]
        pub fn transfer_agent(
            origin: OriginFor<T>,
            agent_id: AgentId,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists and caller is owner
            let mut agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            // Transfer deposit
            T::Currency::repatriate_reserved(
                &who,
                &new_owner,
                T::AgentDeposit::get(),
                frame_support::traits::BalanceStatus::Reserved,
            )?;

            // Update ownership
            let old_owner = agent.owner.clone();
            agent.owner = new_owner.clone();
            Agents::<T>::insert(agent_id, agent);

            // Update agent lists
            AgentsByOwner::<T>::mutate(&old_owner, |agents| {
                if let Some(agent_list) = agents {
                    agent_list.retain(|&id| id != agent_id);
                }
            });

            AgentsByOwner::<T>::try_mutate(&new_owner, |agents| -> DispatchResult {
                let mut agent_list = agents.take().unwrap_or_default();
                agent_list.try_push(agent_id).map_err(|_| Error::<T>::TooManyAgents)?;
                *agents = Some(agent_list);
                Ok(())
            })?;

            Self::deposit_event(Event::AgentTransferred { agent_id, old_owner, new_owner });

            Ok(())
        }

        // ===== Reputation Registry =====

        /// Authorize a client to submit feedback (EIP-8004 compliant)
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent granting authorization
        /// - `client`: Client account being authorized
        /// - `index_limit`: Maximum number of feedbacks allowed
        /// - `expiry_blocks`: Number of blocks until authorization expires
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        /// - `AuthorizationAlreadyExists`: Authorization already exists
        #[pallet::call_index(18)]
        #[pallet::weight(T::WeightInfo::register_agent())]
        pub fn authorize_feedback(
            origin: OriginFor<T>,
            agent_id: AgentId,
            client: T::AccountId,
            index_limit: u32,
            expiry_blocks: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists and caller is owner
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            // Check if authorization already exists
            ensure!(
                !AgentClientAuthorizations::<T>::contains_key(agent_id, &client),
                Error::<T>::AuthorizationAlreadyExists
            );

            // Get next authorization ID
            let auth_id = NextAuthorizationId::<T>::get();
            let next_id = auth_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Calculate expiry block
            let current_block = frame_system::Pallet::<T>::block_number();
            let expiry = current_block.saturating_add(expiry_blocks);

            // Create authorization
            let authorization = FeedbackAuthorization {
                agent_id,
                client: client.clone(),
                index_limit,
                expiry,
                created_at: current_block,
                revoked: false,
            };

            // Store authorization
            FeedbackAuthorizations::<T>::insert(auth_id, authorization);
            AgentClientAuthorizations::<T>::insert(agent_id, &client, auth_id);
            NextAuthorizationId::<T>::put(next_id);

            Self::deposit_event(Event::FeedbackAuthorizationGranted {
                authorization_id: auth_id,
                agent_id,
                client,
                index_limit,
                expiry,
            });

            Ok(())
        }

        /// Revoke a feedback authorization
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent
        /// - `client`: Client whose authorization to revoke
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        /// - `AuthorizationNotFound`: No authorization found
        #[pallet::call_index(19)]
        #[pallet::weight(T::WeightInfo::register_agent())]
        pub fn revoke_authorization(
            origin: OriginFor<T>,
            agent_id: AgentId,
            client: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists and caller is owner
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            // Get authorization ID
            let auth_id = AgentClientAuthorizations::<T>::get(agent_id, &client)
                .ok_or(Error::<T>::AuthorizationNotFound)?;

            // Mark as revoked
            FeedbackAuthorizations::<T>::try_mutate(auth_id, |maybe_auth| -> DispatchResult {
                let auth = maybe_auth.as_mut().ok_or(Error::<T>::AuthorizationNotFound)?;
                auth.revoked = true;
                Ok(())
            })?;

            Self::deposit_event(Event::FeedbackAuthorizationRevoked { authorization_id: auth_id });

            Ok(())
        }

        /// Submit feedback for an agent
        ///
        /// Requires pre-authorization from the agent (EIP-8004 compliant).
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent being reviewed
        /// - `score`: Score (0-100)
        /// - `tags`: Tags for categorization
        /// - `file_uri`: URI pointing to detailed feedback
        /// - `content_hash`: Hash of detailed feedback content
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `NoValidAuthorization`: No valid authorization found
        /// - `AuthorizationExpired`: Authorization has expired
        /// - `AuthorizationRevoked`: Authorization has been revoked
        /// - `AuthorizationIndexLimitExceeded`: Index limit exceeded
        /// - `FeedbackRateLimitExceeded`: Rate limit exceeded
        /// - `InvalidScore`: Score not in range 0-100
        /// - `TooManyTags`: Too many tags provided
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::give_feedback())]
        pub fn give_feedback(
            origin: OriginFor<T>,
            agent_id: AgentId,
            score: u8,
            tags: Vec<Vec<u8>>,
            file_uri: Vec<u8>,
            content_hash: T::Hash,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists
            ensure!(Agents::<T>::contains_key(agent_id), Error::<T>::AgentNotFound);

            let current_block = frame_system::Pallet::<T>::block_number();

            // Verify authorization (EIP-8004 compliant)
            Self::verify_feedback_authorization(agent_id, &who, current_block)?;

            // Check rate limiting
            Self::check_feedback_rate_limit(agent_id, &who, current_block)?;

            // Validate and prepare feedback data
            let (bounded_tags, bounded_uri) = Self::validate_feedback_data(score, tags, file_uri)?;

            // Reserve deposit
            T::Currency::reserve(&who, T::FeedbackDeposit::get())?;

            // Create feedback info
            let feedback = FeedbackInfo {
                agent_id,
                client: who.clone(),
                score,
                tags: bounded_tags,
                file_uri: bounded_uri,
                content_hash,
                created_at: current_block,
                revoked: false,
            };

            // Store feedback and update storage
            let feedback_id = Self::store_feedback(agent_id, &who, feedback, current_block)?;

            // Emit event
            Self::deposit_event(Event::FeedbackGiven { feedback_id, agent_id, client: who, score });

            // Update agent reputation
            Self::update_reputation(agent_id)?;

            Ok(())
        }

        /// Revoke feedback
        ///
        /// # Parameters
        /// - `feedback_id`: ID of the feedback to revoke
        ///
        /// # Errors
        /// - `FeedbackNotFound`: Feedback does not exist
        /// - `NotFeedbackClient`: Caller is not the feedback client
        /// - `FeedbackAlreadyRevoked`: Feedback already revoked
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::revoke_feedback())]
        pub fn revoke_feedback(origin: OriginFor<T>, feedback_id: FeedbackId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify feedback exists and caller is the client
            let mut feedback =
                Feedbacks::<T>::get(feedback_id).ok_or(Error::<T>::FeedbackNotFound)?;
            ensure!(feedback.client == who, Error::<T>::NotFeedbackClient);
            ensure!(!feedback.revoked, Error::<T>::FeedbackAlreadyRevoked);

            // Mark as revoked
            let agent_id = feedback.agent_id;
            feedback.revoked = true;
            Feedbacks::<T>::insert(feedback_id, feedback);

            // Unreserve deposit
            // Note: In rare cases unreserve may return less than requested if account state
            // is corrupted
            let _unreserved = T::Currency::unreserve(&who, T::FeedbackDeposit::get());

            Self::deposit_event(Event::FeedbackRevoked { feedback_id });

            // Update agent reputation (revoked feedback affects score)
            Self::update_reputation(agent_id)?;

            Ok(())
        }

        /// Append a response to feedback
        ///
        /// # Parameters
        /// - `feedback_id`: ID of the feedback
        /// - `response_uri`: URI pointing to response content
        /// - `content_hash`: Hash of response content
        ///
        /// # Errors
        /// - `FeedbackNotFound`: Feedback does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        /// - `TooManyResponses`: Too many responses already
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::append_response())]
        pub fn append_response(
            origin: OriginFor<T>,
            feedback_id: FeedbackId,
            response_uri: Vec<u8>,
            content_hash: T::Hash,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify feedback exists
            let feedback = Feedbacks::<T>::get(feedback_id).ok_or(Error::<T>::FeedbackNotFound)?;

            // Verify caller is the agent owner
            let agent = Agents::<T>::get(feedback.agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            // Validate and bound URI
            let bounded_uri: BoundedVec<u8, T::MaxUriLength> =
                response_uri.try_into().map_err(|_| Error::<T>::UriTooLong)?;

            // Create response
            let response = FeedbackResponse {
                response_uri: bounded_uri,
                content_hash,
                created_at: frame_system::Pallet::<T>::block_number(),
            };

            // Add response to feedback
            FeedbackResponses::<T>::try_mutate(feedback_id, |responses| -> DispatchResult {
                let mut response_list = responses.take().unwrap_or_default();
                response_list.try_push(response).map_err(|_| Error::<T>::TooManyResponses)?;
                *responses = Some(response_list);
                Ok(())
            })?;

            Self::deposit_event(Event::ResponseAppended { feedback_id, responder: who });

            Ok(())
        }

        // ===== Validation Registry =====

        /// Register as a validator
        ///
        /// # Parameters
        /// - `stake`: Amount to stake
        ///
        /// # Errors
        /// - `ValidatorAlreadyRegistered`: Already registered as validator
        /// - `InsufficientStake`: Stake amount below minimum
        /// - `ValidatorNotWhitelisted`: Validator not in whitelist (if
        ///   whitelist is enabled)
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::register_validator())]
        pub fn register_validator(origin: OriginFor<T>, stake: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure not already registered
            ensure!(!Validators::<T>::contains_key(&who), Error::<T>::ValidatorAlreadyRegistered);

            // Check if whitelist is enabled (any entry exists in ValidatorWhitelist)
            // If whitelist exists, ensure the validator is whitelisted
            let whitelist_enabled = ValidatorWhitelist::<T>::iter().next().is_some();
            if whitelist_enabled {
                ensure!(ValidatorWhitelist::<T>::get(&who), Error::<T>::ValidatorNotWhitelisted);
            }

            // Verify minimum stake
            ensure!(stake >= T::ValidatorMinStake::get(), Error::<T>::InsufficientStake);

            // Reserve stake
            T::Currency::reserve(&who, stake)?;

            // Create validator info
            let validator_info = ValidatorInfo {
                account: who.clone(),
                stake,
                registered_at: frame_system::Pallet::<T>::block_number(),
                active: true,
            };

            // Store validator
            Validators::<T>::insert(&who, validator_info);

            Self::deposit_event(Event::ValidatorRegistered { validator: who, stake });

            Ok(())
        }

        /// Unregister as a validator
        ///
        /// # Errors
        /// - `ValidatorNotFound`: Not registered as validator
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::unregister_validator())]
        pub fn unregister_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify validator exists
            let validator = Validators::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;

            // Unreserve stake
            // Note: In rare cases unreserve may return less than requested if account state
            // is corrupted
            let _unreserved = T::Currency::unreserve(&who, validator.stake);

            // Remove validator
            Validators::<T>::remove(&who);

            Self::deposit_event(Event::ValidatorUnregistered { validator: who });

            Ok(())
        }

        /// Request validation for an agent
        ///
        /// # Parameters
        /// - `agent_id`: ID of the agent to validate
        /// - `reward`: Reward amount for validators
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `TooManyValidationRequests`: Too many validation requests for this
        ///   agent
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::request_validation())]
        pub fn request_validation(
            origin: OriginFor<T>,
            agent_id: AgentId,
            reward: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists
            ensure!(Agents::<T>::contains_key(agent_id), Error::<T>::AgentNotFound);

            // Reserve deposit for validation request
            let total_reserve = T::ValidationRequestDeposit::get().saturating_add(reward);
            T::Currency::reserve(&who, total_reserve)?;

            // Get next request ID
            let request_id = NextRequestId::<T>::get();
            let next_id = request_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Calculate deadline
            let current_block = frame_system::Pallet::<T>::block_number();
            let deadline = current_block.saturating_add(T::ValidationDeadline::get());

            // Create validation request
            let request = ValidationRequest {
                agent_id,
                requester: who.clone(),
                reward,
                created_at: current_block,
                deadline,
                completed: false,
            };

            // Store request
            ValidationRequests::<T>::insert(request_id, request);
            NextRequestId::<T>::put(next_id);

            // Update agent validation list
            AgentValidations::<T>::try_mutate(agent_id, |validations| -> DispatchResult {
                let mut validation_list = validations.take().unwrap_or_default();
                validation_list
                    .try_push(request_id)
                    .map_err(|_| Error::<T>::TooManyValidationRequests)?;
                *validations = Some(validation_list);
                Ok(())
            })?;

            Self::deposit_event(Event::ValidationRequested {
                request_id,
                agent_id,
                requester: who,
            });

            Ok(())
        }

        /// Submit validation results
        ///
        /// # Parameters
        /// - `request_id`: ID of the validation request
        /// - `score`: Validation score (0-100)
        /// - `evidence_uri`: URI pointing to evidence
        /// - `content_hash`: Hash of evidence content
        /// - `tags`: Tags for categorization
        ///
        /// # Errors
        /// - `ValidationRequestNotFound`: Request does not exist
        /// - `ValidatorNotFound`: Caller is not a registered validator
        /// - `ValidatorNotActive`: Validator is not active
        /// - `ValidationAlreadySubmitted`: Already submitted validation for
        ///   this request
        /// - `InvalidScore`: Score not in range 0-100
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::submit_validation())]
        pub fn submit_validation(
            origin: OriginFor<T>,
            request_id: RequestId,
            score: u8,
            evidence_uri: Vec<u8>,
            content_hash: T::Hash,
            tags: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify request exists
            ensure!(
                ValidationRequests::<T>::contains_key(request_id),
                Error::<T>::ValidationRequestNotFound
            );

            // Verify caller is an active validator
            let validator = Validators::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;
            ensure!(validator.active, Error::<T>::ValidatorNotActive);

            // Ensure not already submitted
            ensure!(
                !ValidationResponses::<T>::contains_key(request_id, &who),
                Error::<T>::ValidationAlreadySubmitted
            );

            // Validate score
            ensure!(score <= 100, Error::<T>::InvalidScore);

            // Validate and bound tags
            ensure!(tags.len() <= T::MaxTags::get() as usize, Error::<T>::TooManyTags);
            let mut bounded_tags =
                BoundedVec::<BoundedVec<u8, T::MaxTagLength>, T::MaxTags>::default();
            for tag in tags {
                let bounded_tag: BoundedVec<u8, T::MaxTagLength> =
                    tag.try_into().map_err(|_| Error::<T>::TagTooLong)?;
                bounded_tags.try_push(bounded_tag).map_err(|_| Error::<T>::TooManyTags)?;
            }

            // Validate and bound URI
            let bounded_uri: BoundedVec<u8, T::MaxUriLength> =
                evidence_uri.try_into().map_err(|_| Error::<T>::UriTooLong)?;

            // Create validation response
            let response = ValidationResponse {
                request_id,
                validator: who.clone(),
                score,
                evidence_uri: bounded_uri,
                content_hash,
                tags: bounded_tags,
                created_at: frame_system::Pallet::<T>::block_number(),
            };

            // Store validation response
            ValidationResponses::<T>::insert(request_id, &who, response);

            // Get the validation request to access agent_id and reward
            let request = ValidationRequests::<T>::get(request_id)
                .ok_or(Error::<T>::ValidationRequestNotFound)?;

            // Update agent reputation
            Self::update_reputation(request.agent_id)?;

            // Distribute reward to validator if reward is set
            // Use repatriate_reserved for atomic transfer (prevents fund loss if transfer
            // fails)
            if request.reward > BalanceOf::<T>::from(0u32) {
                T::Currency::repatriate_reserved(
                    &request.requester,
                    &who,
                    request.reward,
                    frame_support::traits::BalanceStatus::Free,
                )?;
            }

            // Mark request as completed
            ValidationRequests::<T>::mutate(request_id, |maybe_request| {
                if let Some(req) = maybe_request {
                    req.completed = true;
                }
            });

            Self::deposit_event(Event::ValidationSubmitted { request_id, validator: who, score });

            Ok(())
        }

        // ===== Escrow =====

        /// Create an escrow to pay an agent for a service.
        ///
        /// The escrow follows this timeline:
        /// 1. Creation (block N)
        /// 2. Auto-complete (N + auto_complete_blocks): Agent can claim payment
        /// 3. Timeout (N + timeout, must be > auto_complete_at): Client can
        ///    cancel if agent never claimed
        ///
        /// **Important**: `timeout` must be AFTER `auto_complete_at`. This
        /// ensures:
        /// - Agent gets time to complete work and claim after auto-completion
        /// - Client has a final recourse to cancel if agent never claims
        /// - Agent is protected: once they claim, client cannot cancel
        ///
        /// Example: Set auto_complete_blocks = 7 days, timeout = 14 days
        /// Agent can claim after 7 days, client can cancel after 14 days if
        /// agent didn't claim.
        ///
        /// # Parameters
        /// - `agent_id`: The agent the escrow is for.
        /// - `amount`: The amount to lock in the escrow.
        /// - `timeout`: Block number after which client can cancel if agent
        ///   didn't claim (must be > auto_complete_at).
        /// - `custom_auto_complete_blocks`: Optional custom auto-complete
        ///   duration. If None, uses `EscrowAutoCompleteBlocks`.
        ///
        /// # Errors
        /// - `AgentNotFound`: Agent does not exist
        /// - `InvalidTimeoutPeriod`: Timeout is not after auto_complete_at
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::create_escrow())]
        pub fn create_escrow(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
            timeout: T::BlockNumber,
            custom_auto_complete_blocks: Option<T::BlockNumber>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify agent exists
            ensure!(Agents::<T>::contains_key(agent_id), Error::<T>::AgentNotFound);

            // Reserve funds
            T::Currency::reserve(&who, amount)?;

            // Get next escrow ID
            let escrow_id = NextEscrowId::<T>::get();
            let next_id = escrow_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Calculate auto-complete time (use custom or default)
            let current_block = frame_system::Pallet::<T>::block_number();
            let auto_complete_duration =
                custom_auto_complete_blocks.unwrap_or_else(|| T::EscrowAutoCompleteBlocks::get());
            let auto_complete_at = current_block.saturating_add(auto_complete_duration);

            // Validate timeout: must be after auto_complete_at
            // This gives agent time to claim after auto-complete
            // Client can only cancel if agent fails to claim by timeout
            ensure!(timeout > auto_complete_at, Error::<T>::InvalidTimeoutPeriod);

            // Create escrow info
            let escrow_info = EscrowInfo {
                client: who.clone(),
                agent_id,
                amount,
                auto_complete_at,
                timeout,
                status: EscrowStatus::Active,
                created_at: current_block,
            };

            // Store escrow
            Escrows::<T>::insert(escrow_id, escrow_info);
            NextEscrowId::<T>::put(next_id);

            Self::deposit_event(Event::EscrowCreated {
                escrow_id,
                client: who,
                agent_id,
                amount,
                timeout,
            });

            Ok(())
        }

        /// Claim an escrow as the agent owner.
        ///
        /// The escrow must have auto-completed (after
        /// `EscrowAutoCompleteBlocks` have passed) and not be disputed.
        /// The agent can only claim if the escrow status is
        /// `AutoCompleted` or `Resolved`.
        ///
        /// # Parameters
        /// - `escrow_id`: The ID of the escrow to claim.
        ///
        /// # Errors
        /// - `EscrowNotFound`: Escrow does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        /// - `EscrowNotAutoCompleted`: Escrow has not auto-completed yet
        /// - `EscrowDisputed`: Escrow is disputed and cannot be claimed
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::claim_escrow())]
        pub fn claim_escrow(origin: OriginFor<T>, escrow_id: EscrowId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut escrow = Escrows::<T>::get(escrow_id).ok_or(Error::<T>::EscrowNotFound)?;
            let agent = Agents::<T>::get(escrow.agent_id).ok_or(Error::<T>::AgentNotFound)?;

            // Ensure caller is the agent owner
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            let current_block = frame_system::Pallet::<T>::block_number();

            // Check if escrow can be auto-completed
            if escrow.status == EscrowStatus::Active {
                // Check if auto-complete time has passed
                if current_block >= escrow.auto_complete_at {
                    // Auto-complete the escrow
                    escrow.status = EscrowStatus::AutoCompleted;
                    Escrows::<T>::insert(escrow_id, &escrow);
                    Self::deposit_event(Event::EscrowAutoCompleted { escrow_id });
                } else {
                    return Err(Error::<T>::EscrowNotAutoCompleted.into());
                }
            }

            // Ensure escrow is not disputed
            ensure!(escrow.status != EscrowStatus::Disputed, Error::<T>::EscrowDisputed);

            // Ensure escrow is in claimable status
            ensure!(
                escrow.status == EscrowStatus::AutoCompleted
                    || escrow.status == EscrowStatus::Resolved,
                Error::<T>::EscrowNotAutoCompleted
            );

            // Unreserve from client and transfer to agent owner
            T::Currency::unreserve(&escrow.client, escrow.amount);
            T::Currency::transfer(
                &escrow.client,
                &agent.owner,
                escrow.amount,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Remove escrow from storage
            Escrows::<T>::remove(escrow_id);

            Self::deposit_event(Event::EscrowClaimed { escrow_id, agent_id: escrow.agent_id });

            Ok(())
        }

        /// Cancel an escrow as the client after the timeout.
        ///
        /// The client can only cancel if:
        /// 1. The timeout period has been reached, AND
        /// 2. The escrow status is still Active (agent hasn't claimed)
        ///
        /// Timeline: created → auto_complete_at → timeout
        /// - Before auto_complete_at: Agent working, neither can cancel/claim
        /// - After auto_complete_at: Agent can claim
        /// - After timeout: Client can cancel IF agent didn't claim yet
        ///
        /// If the agent claims before timeout, the escrow is removed and client
        /// cannot cancel. This protects the agent once they've claimed.
        ///
        /// # Parameters
        /// - `escrow_id`: The ID of the escrow to cancel.
        ///
        /// # Errors
        /// - `EscrowNotFound`: Escrow does not exist (or already claimed)
        /// - `NotEscrowClient`: Caller is not the escrow client
        /// - `EscrowNotTimedOut`: Timeout period has not been reached
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::cancel_escrow())]
        pub fn cancel_escrow(origin: OriginFor<T>, escrow_id: EscrowId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let escrow = Escrows::<T>::get(escrow_id).ok_or(Error::<T>::EscrowNotFound)?;

            // Ensure caller is the client
            ensure!(escrow.client == who, Error::<T>::NotEscrowClient);

            let current_block = frame_system::Pallet::<T>::block_number();

            // Ensure timeout has passed
            ensure!(current_block >= escrow.timeout, Error::<T>::EscrowNotTimedOut);

            // If we get here, the escrow still exists (agent didn't claim)
            // and timeout passed, so client can cancel

            // Unreserve funds back to the client
            T::Currency::unreserve(&escrow.client, escrow.amount);

            // Remove escrow from storage
            Escrows::<T>::remove(escrow_id);

            Self::deposit_event(Event::EscrowCancelled { escrow_id, client: who });

            Ok(())
        }

        /// Dispute an escrow as the client
        ///
        /// The client can dispute an escrow before it auto-completes if they
        /// believe the service was not provided correctly. This prevents the
        /// agent from claiming the funds until the dispute is resolved.
        ///
        /// # Parameters
        /// - `escrow_id`: The ID of the escrow to dispute
        ///
        /// # Errors
        /// - `EscrowNotFound`: Escrow does not exist
        /// - `NotEscrowClient`: Caller is not the escrow client
        /// - `CannotDisputeEscrow`: Escrow is not in Active or AutoCompleted
        ///   status
        #[pallet::call_index(20)]
        #[pallet::weight(T::WeightInfo::create_escrow())]
        pub fn dispute_escrow(origin: OriginFor<T>, escrow_id: EscrowId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut escrow = Escrows::<T>::get(escrow_id).ok_or(Error::<T>::EscrowNotFound)?;

            // Ensure caller is the client
            ensure!(escrow.client == who, Error::<T>::NotEscrowClient);

            // Can only dispute Active or AutoCompleted escrows
            ensure!(
                escrow.status == EscrowStatus::Active
                    || escrow.status == EscrowStatus::AutoCompleted,
                Error::<T>::CannotDisputeEscrow
            );

            // Change status to Disputed
            escrow.status = EscrowStatus::Disputed;
            Escrows::<T>::insert(escrow_id, escrow);

            Self::deposit_event(Event::EscrowDisputed { escrow_id, disputer: who });

            Ok(())
        }

        /// Resolve an escrow dispute
        ///
        /// Only the dispute resolver (e.g., Root or Council) can resolve escrow
        /// disputes. The resolution can either favor the client (refund) or the
        /// agent (allow claim).
        ///
        /// # Parameters
        /// - `escrow_id`: The ID of the escrow with dispute
        /// - `favor_client`: If true, refund to client; if false, allow agent
        ///   to claim
        ///
        /// # Errors
        /// - `EscrowNotFound`: Escrow does not exist
        /// - `CannotDisputeEscrow`: Escrow is not disputed
        #[pallet::call_index(21)]
        #[pallet::weight(T::WeightInfo::resolve_dispute())]
        pub fn resolve_escrow_dispute(
            origin: OriginFor<T>,
            escrow_id: EscrowId,
            favor_client: bool,
        ) -> DispatchResult {
            // Ensure origin has dispute resolver permission
            T::DisputeResolverOrigin::ensure_origin(origin)?;

            let mut escrow = Escrows::<T>::get(escrow_id).ok_or(Error::<T>::EscrowNotFound)?;

            // Ensure escrow is disputed
            ensure!(escrow.status == EscrowStatus::Disputed, Error::<T>::CannotDisputeEscrow);

            if favor_client {
                // Refund to client
                T::Currency::unreserve(&escrow.client, escrow.amount);
                Escrows::<T>::remove(escrow_id);
                Self::deposit_event(Event::EscrowCancelled { escrow_id, client: escrow.client });
            } else {
                // Mark as resolved, allowing agent to claim
                escrow.status = EscrowStatus::Resolved;
                Escrows::<T>::insert(escrow_id, &escrow);
                Self::deposit_event(Event::EscrowDisputeResolved {
                    escrow_id,
                    status: EscrowStatus::Resolved,
                });
            }

            Ok(())
        }

        /// Cancel a validation request after deadline
        ///
        /// # Parameters
        /// - `request_id`: ID of the validation request to cancel
        ///
        /// # Errors
        /// - `ValidationRequestNotFound`: Request does not exist
        /// - `NotRequester`: Caller is not the requester
        /// - `ValidationRequestCompleted`: Request has already been completed
        /// - `ValidationDeadlineNotPassed`: Deadline has not passed yet
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::cancel_validation_request())]
        pub fn cancel_validation_request(
            origin: OriginFor<T>,
            request_id: RequestId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify request exists
            let request = ValidationRequests::<T>::get(request_id)
                .ok_or(Error::<T>::ValidationRequestNotFound)?;

            // Ensure caller is the requester
            ensure!(request.requester == who, Error::<T>::NotRequester);

            // Ensure request is not completed
            ensure!(!request.completed, Error::<T>::ValidationRequestCompleted);

            // Ensure deadline has passed
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block >= request.deadline, Error::<T>::ValidationDeadlineNotPassed);

            // Unreserve deposit and reward
            let total_reserve = T::ValidationRequestDeposit::get().saturating_add(request.reward);
            T::Currency::unreserve(&who, total_reserve);

            // Remove the request id from the agent's validation list to avoid stale
            // references
            AgentValidations::<T>::mutate(request.agent_id, |maybe_list| {
                if let Some(list) = maybe_list.as_mut() {
                    if let Some(pos) = list.iter().position(|id| *id == request_id) {
                        list.swap_remove(pos);
                    }
                }
            });

            // Remove request from storage
            ValidationRequests::<T>::remove(request_id);

            Self::deposit_event(Event::ValidationRequestCancelled { request_id, requester: who });

            Ok(())
        }

        /// Dispute a feedback entry
        ///
        /// # Parameters
        /// - `feedback_id`: ID of the feedback to dispute
        /// - `reason_uri`: URI pointing to dispute reason
        /// - `content_hash`: Hash of dispute reason content
        ///
        /// # Errors
        /// - `FeedbackNotFound`: Feedback does not exist
        /// - `NotAgentOwner`: Caller is not the agent owner
        /// - `CannotDisputeRevokedFeedback`: Cannot dispute revoked feedback
        /// - `TooManyDisputes`: Too many disputes for this feedback
        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::dispute_feedback())]
        pub fn dispute_feedback(
            origin: OriginFor<T>,
            feedback_id: FeedbackId,
            reason_uri: Vec<u8>,
            content_hash: T::Hash,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify feedback exists
            let feedback = Feedbacks::<T>::get(feedback_id).ok_or(Error::<T>::FeedbackNotFound)?;

            // Ensure feedback is not revoked
            ensure!(!feedback.revoked, Error::<T>::CannotDisputeRevokedFeedback);

            // Verify caller is the agent owner
            let agent = Agents::<T>::get(feedback.agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.owner == who, Error::<T>::NotAgentOwner);

            // Validate and bound URI
            let bounded_uri: BoundedVec<u8, T::MaxUriLength> =
                reason_uri.try_into().map_err(|_| Error::<T>::UriTooLong)?;

            // Reserve dispute deposit
            T::Currency::reserve(&who, T::DisputeDeposit::get())?;

            // Get next dispute ID
            let dispute_id = NextDisputeId::<T>::get();
            let next_id = dispute_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Create dispute
            let dispute = DisputeInfo {
                feedback_id,
                disputer: who.clone(),
                reason_uri: bounded_uri,
                content_hash,
                status: DisputeStatus::Open,
                created_at: frame_system::Pallet::<T>::block_number(),
                resolved_at: None,
            };

            // Store dispute
            Disputes::<T>::insert(dispute_id, dispute);
            NextDisputeId::<T>::put(next_id);

            // Update feedback disputes list
            FeedbackDisputes::<T>::try_mutate(feedback_id, |disputes| -> DispatchResult {
                let mut dispute_list = disputes.take().unwrap_or_default();
                dispute_list.try_push(dispute_id).map_err(|_| Error::<T>::TooManyDisputes)?;
                *disputes = Some(dispute_list);
                Ok(())
            })?;

            Self::deposit_event(Event::DisputeCreated { dispute_id, feedback_id, disputer: who });

            Ok(())
        }

        /// Resolve a dispute
        ///
        /// # Parameters
        /// - `dispute_id`: ID of the dispute to resolve
        /// - `status`: Resolution status
        ///
        /// # Errors
        /// - `DisputeNotFound`: Dispute does not exist
        /// - `DisputeAlreadyResolved`: Dispute has already been resolved
        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::resolve_dispute())]
        pub fn resolve_dispute(
            origin: OriginFor<T>,
            dispute_id: DisputeId,
            status: DisputeStatus,
        ) -> DispatchResult {
            // Ensure origin has dispute resolver permission
            T::DisputeResolverOrigin::ensure_origin(origin)?;

            // Verify dispute exists
            let mut dispute = Disputes::<T>::get(dispute_id).ok_or(Error::<T>::DisputeNotFound)?;

            // Ensure dispute is not already resolved
            ensure!(dispute.status == DisputeStatus::Open, Error::<T>::DisputeAlreadyResolved);

            // Update dispute status
            dispute.status = status.clone();
            dispute.resolved_at = Some(frame_system::Pallet::<T>::block_number());
            Disputes::<T>::insert(dispute_id, &dispute);

            // Handle resolution based on status
            match status {
                DisputeStatus::ResolvedForDisputer => {
                    // Revoke the disputed feedback
                    if let Some(mut feedback) = Feedbacks::<T>::get(dispute.feedback_id) {
                        feedback.revoked = true;
                        Feedbacks::<T>::insert(dispute.feedback_id, &feedback);

                        // Update reputation
                        Self::update_reputation(feedback.agent_id)?;

                        // Return deposit to disputer
                        T::Currency::unreserve(&dispute.disputer, T::DisputeDeposit::get());
                    }
                }
                DisputeStatus::ResolvedAgainstDisputer | DisputeStatus::Dismissed => {
                    // Slash disputer's deposit (keep it reserved or slash it)
                    // For now, we'll keep it reserved as a penalty
                    // In a production system, this could be slashed to treasury
                }
                DisputeStatus::Open => {
                    // This should not happen as we're resolving
                }
            }

            Self::deposit_event(Event::DisputeResolved { dispute_id, status });

            Ok(())
        }

        /// Add a validator to the whitelist
        ///
        /// # Parameters
        /// - `validator`: Account to add to whitelist
        ///
        /// # Errors
        /// - `ValidatorAlreadyWhitelisted`: Validator already in whitelist
        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::add_validator_to_whitelist())]
        pub fn add_validator_to_whitelist(
            origin: OriginFor<T>,
            validator: T::AccountId,
        ) -> DispatchResult {
            // Ensure origin has validator manager permission
            T::ValidatorManagerOrigin::ensure_origin(origin)?;

            // Ensure not already whitelisted
            ensure!(
                !ValidatorWhitelist::<T>::get(&validator),
                Error::<T>::ValidatorAlreadyWhitelisted
            );

            // Add to whitelist
            ValidatorWhitelist::<T>::insert(&validator, true);

            Self::deposit_event(Event::ValidatorWhitelisted { validator });

            Ok(())
        }

        /// Remove a validator from the whitelist
        ///
        /// # Parameters
        /// - `validator`: Account to remove from whitelist
        ///
        /// # Errors
        /// - `ValidatorNotWhitelisted`: Validator not in whitelist
        #[pallet::call_index(17)]
        #[pallet::weight(T::WeightInfo::remove_validator_from_whitelist())]
        pub fn remove_validator_from_whitelist(
            origin: OriginFor<T>,
            validator: T::AccountId,
        ) -> DispatchResult {
            // Ensure origin has validator manager permission
            T::ValidatorManagerOrigin::ensure_origin(origin)?;

            // Ensure is whitelisted
            ensure!(ValidatorWhitelist::<T>::get(&validator), Error::<T>::ValidatorNotWhitelisted);

            // Remove from whitelist
            ValidatorWhitelist::<T>::remove(&validator);

            Self::deposit_event(Event::ValidatorRemovedFromWhitelist { validator });

            Ok(())
        }
    }

    // ===== Helper Functions =====

    impl<T: Config> Pallet<T> {
        /// Verify EIP-8004 feedback authorization
        ///
        /// Checks if the client has valid authorization from the agent
        /// to submit feedback, including expiry and index limits.
        fn verify_feedback_authorization(
            agent_id: AgentId,
            client: &T::AccountId,
            current_block: T::BlockNumber,
        ) -> DispatchResult {
            // Get authorization
            let auth_id = AgentClientAuthorizations::<T>::get(agent_id, client)
                .ok_or(Error::<T>::NoValidAuthorization)?;

            let authorization = FeedbackAuthorizations::<T>::get(auth_id)
                .ok_or(Error::<T>::NoValidAuthorization)?;

            // Check if authorization is revoked
            ensure!(!authorization.revoked, Error::<T>::AuthorizationRevoked);

            // Check if authorization has expired
            ensure!(current_block <= authorization.expiry, Error::<T>::AuthorizationExpired);

            // Check index limit
            let current_index = FeedbackIndices::<T>::get(agent_id, client);
            ensure!(
                current_index < authorization.index_limit,
                Error::<T>::AuthorizationIndexLimitExceeded
            );

            // Prevent overflow: ensure we can safely increment the index
            ensure!(current_index < u32::MAX, Error::<T>::TooManyFeedbacks);

            Ok(())
        }

        /// Check feedback rate limiting
        ///
        /// Ensures sufficient time has passed since last feedback from this
        /// client to this agent.
        fn check_feedback_rate_limit(
            agent_id: AgentId,
            client: &T::AccountId,
            current_block: T::BlockNumber,
        ) -> DispatchResult {
            if let Some(last_timestamp) = LastFeedbackTimestamp::<T>::get(agent_id, client) {
                let blocks_passed = current_block.saturating_sub(last_timestamp);
                ensure!(
                    blocks_passed >= T::FeedbackRateLimitBlocks::get(),
                    Error::<T>::FeedbackRateLimitExceeded
                );
            }
            Ok(())
        }

        /// Validate and prepare feedback data
        ///
        /// Validates score, tags, and URI, returning bounded vectors.
        fn validate_feedback_data(
            score: u8,
            tags: Vec<Vec<u8>>,
            file_uri: Vec<u8>,
        ) -> Result<
            (
                BoundedVec<BoundedVec<u8, T::MaxTagLength>, T::MaxTags>,
                BoundedVec<u8, T::MaxUriLength>,
            ),
            DispatchError,
        > {
            // Validate score
            ensure!(score <= 100, Error::<T>::InvalidScore);

            // Validate and bound tags
            ensure!(tags.len() <= T::MaxTags::get() as usize, Error::<T>::TooManyTags);
            let mut bounded_tags =
                BoundedVec::<BoundedVec<u8, T::MaxTagLength>, T::MaxTags>::default();
            for tag in tags {
                let bounded_tag: BoundedVec<u8, T::MaxTagLength> =
                    tag.try_into().map_err(|_| Error::<T>::TagTooLong)?;
                bounded_tags.try_push(bounded_tag).map_err(|_| Error::<T>::TooManyTags)?;
            }

            // Validate and bound URI
            let bounded_uri: BoundedVec<u8, T::MaxUriLength> =
                file_uri.try_into().map_err(|_| Error::<T>::UriTooLong)?;

            Ok((bounded_tags, bounded_uri))
        }

        /// Store feedback and update related storage
        ///
        /// Creates the feedback entry and updates all related storage items.
        fn store_feedback(
            agent_id: AgentId,
            client: &T::AccountId,
            feedback_info: FeedbackInfo<T>,
            current_block: T::BlockNumber,
        ) -> Result<FeedbackId, DispatchError> {
            // Get next feedback ID
            let feedback_id = NextFeedbackId::<T>::get();
            let next_id = feedback_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Store feedback
            Feedbacks::<T>::insert(feedback_id, feedback_info);
            NextFeedbackId::<T>::put(next_id);

            // Update agent feedback list
            AgentFeedbacks::<T>::try_mutate(agent_id, |feedbacks| -> DispatchResult {
                let mut feedback_list = feedbacks.take().unwrap_or_default();
                feedback_list.try_push(feedback_id).map_err(|_| Error::<T>::TooManyFeedbacks)?;
                *feedbacks = Some(feedback_list);
                Ok(())
            })?;

            // Update client feedback list
            ClientFeedbacks::<T>::try_mutate(client, |feedbacks| -> DispatchResult {
                let mut feedback_list = feedbacks.take().unwrap_or_default();
                feedback_list.try_push(feedback_id).map_err(|_| Error::<T>::TooManyFeedbacks)?;
                *feedbacks = Some(feedback_list);
                Ok(())
            })?;

            // Update feedback index and timestamp
            FeedbackIndices::<T>::try_mutate(agent_id, client, |index| -> DispatchResult {
                *index = index.checked_add(1).ok_or(Error::<T>::TooManyFeedbacks)?;
                Ok(())
            })?;
            LastFeedbackTimestamp::<T>::insert(agent_id, client, current_block);

            Ok(feedback_id)
        }

        /// Calculate reputation score for an agent
        ///
        /// This function computes the reputation based on:
        /// - Average feedback score
        /// - Number of feedback entries
        /// - Average validation score
        /// - Number of validation entries
        ///
        /// Returns a ReputationScore struct
        pub fn calculate_reputation(agent_id: AgentId) -> Result<ReputationScore, DispatchError> {
            let current_block = frame_system::Pallet::<T>::block_number();

            // Calculate feedback score with time decay
            let (feedback_score, feedback_count) = if let Some(feedback_ids) =
                AgentFeedbacks::<T>::get(agent_id)
            {
                let mut weighted_score: u64 = 0;
                let mut total_weight: u64 = 0;
                let mut valid_count: u32 = 0;

                for feedback_id in feedback_ids.iter() {
                    if let Some(feedback) = Feedbacks::<T>::get(feedback_id) {
                        // Only count non-revoked feedback
                        if !feedback.revoked {
                            // Calculate age in blocks
                            let age = current_block.saturating_sub(feedback.created_at);

                            // Convert BlockNumber to u32 for comparison
                            let age_u32 = TryInto::<u32>::try_into(age).unwrap_or(u32::MAX);

                            // Time decay weights:
                            // 0-30 days (assuming ~7200 blocks/day): 100%
                            // 31-90 days: 70%
                            // 91-180 days: 40%
                            // 180+ days: 20%
                            let days_in_blocks = 7200u32; // ~1 block per 12 seconds
                            let weight = if age_u32 <= 30 * days_in_blocks {
                                100u64 // 0-30 days: 100%
                            } else if age_u32 <= 90 * days_in_blocks {
                                70u64 // 31-90 days: 70%
                            } else if age_u32 <= 180 * days_in_blocks {
                                40u64 // 91-180 days: 40%
                            } else {
                                20u64 // 180+ days: 20%
                            };

                            // Apply weight to score
                            let weighted_feedback_score =
                                (feedback.score as u64).saturating_mul(weight);
                            weighted_score = weighted_score.saturating_add(weighted_feedback_score);
                            total_weight = total_weight.saturating_add(weight);
                            valid_count = valid_count.saturating_add(1);
                        }
                    }
                }

                let avg_score = if total_weight > 0 {
                    // Calculate weighted average and scale to 0-10000
                    // Use saturating_mul to prevent overflow, then clamp to max value
                    let score = weighted_score.saturating_mul(100) / total_weight;
                    // Ensure score doesn't exceed 10000 (representing 100.00%)
                    score.min(10000) as u32
                } else {
                    0
                };

                (avg_score, valid_count)
            } else {
                (0, 0)
            };

            // Calculate validation score
            let (validation_score, validation_count) =
                if let Some(request_ids) = AgentValidations::<T>::get(agent_id) {
                    let mut total_score: u64 = 0;
                    let mut valid_count: u32 = 0;

                    for request_id in request_ids.iter() {
                        // Iterate through all validation responses for this request
                        for (_, response) in ValidationResponses::<T>::iter_prefix(request_id) {
                            total_score = total_score.saturating_add(response.score as u64);
                            valid_count = valid_count.saturating_add(1);
                        }
                    }

                    let avg_score = if valid_count > 0 {
                        // Scale to 0-10000 (representing 0.00-100.00)
                        // Use saturating_mul to prevent overflow
                        let score = total_score.saturating_mul(100) / valid_count as u64;
                        // Ensure score doesn't exceed 10000
                        score.min(10000) as u32
                    } else {
                        0
                    };

                    (avg_score, valid_count)
                } else {
                    (0, 0)
                };

            // Calculate overall weighted score
            // Weight: 60% feedback, 40% validation
            let overall_score = if feedback_count > 0 || validation_count > 0 {
                let feedback_weight = 60u32;
                let validation_weight = 40u32;

                let weighted_feedback = feedback_score.saturating_mul(feedback_weight) / 100;
                let weighted_validation = validation_score.saturating_mul(validation_weight) / 100;

                weighted_feedback.saturating_add(weighted_validation)
            } else {
                0
            };

            // Convert block number to u32 for storage
            let last_updated = TryInto::<u32>::try_into(current_block).unwrap_or(0);

            Ok(ReputationScore {
                feedback_score,
                feedback_count,
                validation_score,
                validation_count,
                overall_score,
                last_updated,
            })
        }

        /// Update reputation score for an agent
        ///
        /// Recalculates and stores the updated reputation score
        pub fn update_reputation(agent_id: AgentId) -> DispatchResult {
            let reputation = Self::calculate_reputation(agent_id)?;
            let overall_score = reputation.overall_score;

            AgentReputations::<T>::insert(agent_id, reputation);

            Self::deposit_event(Event::ReputationUpdated { agent_id, overall_score });

            Ok(())
        }

        /// Get paginated feedbacks for an agent
        ///
        /// Returns a slice of feedback IDs and FeedbackInfo for the specified
        /// page. This is useful for off-chain reading to avoid loading all
        /// feedbacks at once.
        ///
        /// # Parameters
        /// - `agent_id`: The agent to query feedbacks for
        /// - `page`: Page number (0-indexed)
        /// - `page_size`: Number of items per page
        ///
        /// # Returns
        /// - Vec of (FeedbackId, FeedbackInfo) tuples
        /// - Total count of feedbacks
        pub fn get_agent_feedbacks_paginated(
            agent_id: AgentId,
            page: u32,
            page_size: u32,
        ) -> (Vec<(FeedbackId, FeedbackInfo<T>)>, u32) {
            let feedback_ids = AgentFeedbacks::<T>::get(agent_id).unwrap_or_default();
            let total_count = feedback_ids.len() as u32;

            let start = (page * page_size) as usize;
            let end = ((page + 1) * page_size).min(total_count) as usize;

            let mut results = Vec::new();
            for i in start..end {
                if let Some(&feedback_id) = feedback_ids.get(i) {
                    if let Some(feedback_info) = Feedbacks::<T>::get(feedback_id) {
                        results.push((feedback_id, feedback_info));
                    }
                }
            }

            (results, total_count)
        }

        /// Get paginated validation requests for an agent
        ///
        /// Returns a slice of validation request IDs and ValidationRequest
        /// for the specified page.
        ///
        /// # Parameters
        /// - `agent_id`: The agent to query validations for
        /// - `page`: Page number (0-indexed)
        /// - `page_size`: Number of items per page
        ///
        /// # Returns
        /// - Vec of (RequestId, ValidationRequest) tuples
        /// - Total count of validation requests
        pub fn get_agent_validations_paginated(
            agent_id: AgentId,
            page: u32,
            page_size: u32,
        ) -> (Vec<(RequestId, ValidationRequest<T>)>, u32) {
            let request_ids = AgentValidations::<T>::get(agent_id).unwrap_or_default();
            let total_count = request_ids.len() as u32;

            let start = (page * page_size) as usize;
            let end = ((page + 1) * page_size).min(total_count) as usize;

            let mut results = Vec::new();
            for i in start..end {
                if let Some(&request_id) = request_ids.get(i) {
                    if let Some(request_info) = ValidationRequests::<T>::get(request_id) {
                        results.push((request_id, request_info));
                    }
                }
            }

            (results, total_count)
        }

        /// Get feedbacks for an agent by client
        ///
        /// Returns all feedbacks given by a specific client to an agent.
        /// Useful for checking feedback history between a client-agent pair.
        ///
        /// # Parameters
        /// - `agent_id`: The agent to query
        /// - `client`: The client who provided feedbacks
        ///
        /// # Returns
        /// - Vec of (FeedbackId, FeedbackInfo) tuples
        pub fn get_feedbacks_by_client(
            agent_id: AgentId,
            client: &T::AccountId,
        ) -> Vec<(FeedbackId, FeedbackInfo<T>)> {
            let feedback_ids = AgentFeedbacks::<T>::get(agent_id).unwrap_or_default();
            let mut results = Vec::new();

            for &feedback_id in feedback_ids.iter() {
                if let Some(feedback_info) = Feedbacks::<T>::get(feedback_id) {
                    if &feedback_info.client == client {
                        results.push((feedback_id, feedback_info));
                    }
                }
            }

            results
        }
    }
}
