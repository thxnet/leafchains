//! State-filtering helpers for fork-based leafchain chain specs.
//!
//! [`filter_forked_storage`] removes consensus- and block-execution-transient
//! keys from a raw genesis storage snapshot so the resulting chain spec starts
//! from a clean, deterministic state.
//!
//! ## Drop logic (three tiers, evaluated in order)
//!
//! 1. **Exact-match**: `:extrinsic_index`, `:intrablock_entropy` — checked
//!    first; short keys exit early on length mismatch, so O(1) per entry.
//! 2. **Pallet-level** (16-byte twox_128 prefix): any key whose first 16 bytes
//!    match `twox_128(pallet_name)` for a listed pallet is dropped entirely.
//! 3. **Item-level** (32-byte prefix): any key whose first 32 bytes match
//!    `twox_128(pallet) || twox_128(item)` for a listed (pallet, item) pair
//!    is dropped.
//!
//! Everything else — including `System.Account`, `:code`, `:heappages`,
//! `ParachainInfo.ParachainId` — is preserved verbatim.
//! `children_default` is passed through untouched.

use cumulus_primitives_core::ParaId;
use general_runtime::{AccountId, AuraId, Balance, UNITS};
use sp_core::{hashing::twox_128, Pair, Public};
use sp_runtime::traits::IdentifyAccount;

// ---------------------------------------------------------------------------
// DROP table — pallet-level (whole-pallet wipe)
// ---------------------------------------------------------------------------

/// Pallets whose *entire* storage subtree is dropped.
///
/// Each entry is a raw `&[u8]` pallet name; the 16-byte twox_128 prefix is
/// computed at call-time inside `filter_forked_storage`.
///
/// # Verification
///
/// All 8 identifiers are confirmed present in the leafchain `general_runtime`
/// `construct_runtime!` macro at runtime/general/src/lib.rs lines 861–906:
///   Aura              → pallet_aura         = 23
///   AuraExt           → cumulus_pallet_aura_ext = 24
///   Authorship        → pallet_authorship    = 20
///   CollatorSelection → pallet_collator_selection = 21
///   CumulusXcm        → cumulus_pallet_xcm   = 32
///   DmpQueue          → cumulus_pallet_dmp_queue = 33
///   Session           → pallet_session       = 22
///   XcmpQueue         → cumulus_pallet_xcmp_queue = 30
///
/// Sorted lexicographically (ASCII byte order).
static LEAFCHAIN_DROP_PALLETS: &[&[u8]] = &[
	b"Aura",
	b"AuraExt",
	b"Authorship",
	b"CollatorSelection",
	b"CumulusXcm",
	b"DmpQueue",
	b"Session",
	b"XcmpQueue",
];

// ---------------------------------------------------------------------------
// DROP table — item-level (specific storage items)
// ---------------------------------------------------------------------------

/// Specific storage items to drop, given as `(pallet_name, item_name)` pairs.
///
/// The 32-byte prefix `twox_128(pallet) || twox_128(item)` is computed at
/// call-time inside `filter_forked_storage`.
///
/// # Layout
///
/// 14 ParachainSystem items (relay-chain-derived transient state, rebuilt on
///   first relay-chain block after fork):
///     HostConfiguration, HrmpWatermarks, LastDmqMqcHead, LastHrmpMqcHeads,
///     LastRelayChainBlockNumber, NewValidationCode, PendingValidationCode,
///     ProcessedDownwardMessages, RelayStateProof, RelevantMessagingState,
///     UpgradeGoAhead, UpgradeRestrictionSignal, UpwardMessages,
///     ValidationData
///
/// 10 System items (block-scoped transient; reset by frame_system
///   on_initialize/on_finalize):
///     AllExtrinsicsLen, BlockHash, BlockWeight, EventCount, Events,
///     ExecutionPhase, ExtrinsicCount, ExtrinsicData, LastRuntimeUpgrade,
///     Number
///
/// 2 Timestamp items (always re-derived from the first inherent):
///     DidUpdate, Now
///
/// Sorted lexicographically by `(pallet, item)` pair (ASCII byte order).
/// Note: 'P' (0x50) < 'S' (0x53) < 'T' (0x54), so ParachainSystem entries
/// come first, then System, then Timestamp.
static LEAFCHAIN_DROP_ITEMS: &[(&[u8], &[u8])] = &[
	// --- ParachainSystem (14 items) ---
	(b"ParachainSystem", b"HostConfiguration"),
	(b"ParachainSystem", b"HrmpWatermarks"),
	(b"ParachainSystem", b"LastDmqMqcHead"),
	(b"ParachainSystem", b"LastHrmpMqcHeads"),
	(b"ParachainSystem", b"LastRelayChainBlockNumber"),
	(b"ParachainSystem", b"NewValidationCode"),
	(b"ParachainSystem", b"PendingValidationCode"),
	(b"ParachainSystem", b"ProcessedDownwardMessages"),
	(b"ParachainSystem", b"RelayStateProof"),
	(b"ParachainSystem", b"RelevantMessagingState"),
	(b"ParachainSystem", b"UpgradeGoAhead"),
	(b"ParachainSystem", b"UpgradeRestrictionSignal"),
	(b"ParachainSystem", b"UpwardMessages"),
	(b"ParachainSystem", b"ValidationData"),
	// --- System (10 items) ---
	(b"System", b"AllExtrinsicsLen"),
	(b"System", b"BlockHash"),
	(b"System", b"BlockWeight"),
	(b"System", b"EventCount"),
	(b"System", b"Events"),
	(b"System", b"ExecutionPhase"),
	(b"System", b"ExtrinsicCount"),
	(b"System", b"ExtrinsicData"),
	(b"System", b"LastRuntimeUpgrade"),
	(b"System", b"Number"),
	// --- Timestamp (2 items) ---
	(b"Timestamp", b"DidUpdate"),
	(b"Timestamp", b"Now"),
];

// ---------------------------------------------------------------------------
// DROP table — well-known exact-match keys
// ---------------------------------------------------------------------------

/// Bare well-known keys dropped by exact equality (not prefix).
///
/// Sorted lexicographically (ASCII byte order).
static LEAFCHAIN_DROP_EXACT: &[&[u8]] = &[b":extrinsic_index", b":intrablock_entropy"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Remove consensus- and block-execution-transient storage entries from a raw
/// leafchain genesis `Storage`, returning the cleaned copy.
///
/// # Guarantees
///
/// - Pure: no I/O, no logging, no global mutation.
/// - Infallible: always returns a valid `Storage`.
/// - `children_default` is forwarded byte-for-byte.
/// - `ParachainInfo.ParachainId`, `System.Account`, `:code`, `:heappages`
///   are all preserved (not in any drop table).
pub fn filter_forked_storage(storage: sp_core::storage::Storage) -> sp_core::storage::Storage {
	// Compute pallet-level 16-byte prefixes locally.
	// Allocated once per call; filter_forked_storage is a one-shot fork utility,
	// not a hot path, so the allocation cost is immaterial.
	let pallet_prefixes: Vec<[u8; 16]> =
		LEAFCHAIN_DROP_PALLETS.iter().map(|name| twox_128(name)).collect();

	// Compute item-level 32-byte prefixes locally.
	let item_prefixes: Vec<[u8; 32]> = LEAFCHAIN_DROP_ITEMS
		.iter()
		.map(|(pallet, item)| {
			let mut prefix = [0u8; 32];
			prefix[..16].copy_from_slice(&twox_128(pallet));
			prefix[16..].copy_from_slice(&twox_128(item));
			prefix
		})
		.collect();

	// Filter top-level storage.
	//
	// A key is DROPPED if ANY of the following hold:
	//   (a) its first 16 bytes match a pallet prefix, OR
	//   (b) its first 32 bytes match an item prefix, OR
	//   (c) it exactly equals a well-known drop key.
	//
	// Otherwise it is KEPT.
	let top = storage
		.top
		.into_iter()
		.filter(|(key, _value)| !should_drop(key, &pallet_prefixes, &item_prefixes))
		.collect();

	// children_default is passed through verbatim; parachain child-trie data
	// is unrelated to the consensus/block-transient keys being filtered above.
	sp_core::storage::Storage { top, children_default: storage.children_default }
}

// ---------------------------------------------------------------------------
// Fork genesis builder types
// ---------------------------------------------------------------------------

/// A collator entry for a fork genesis: `(account_id, initial_balance, aura_key)`.
///
/// The triple mirrors the `invulnerables` tuple used by `testnet_genesis` and
/// `mainnet_genesis` so that callers can reuse existing collator-construction
/// utilities without adaptation.
pub type CollatorTuple = (AccountId, Balance, AuraId);

// ---------------------------------------------------------------------------
// Fork genesis builder helpers
// ---------------------------------------------------------------------------

/// Derive an application-key public key from a well-known development seed.
///
/// Mirrors the `get_from_seed` helper used in node-template chain specs.
/// Panics only if the seed is syntactically invalid — acceptable for static
/// dev seeds that are never user-supplied.
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static dev seed is valid; qed")
		.public()
}

/// Derive an `AccountId` from a well-known development seed.
fn account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	sp_runtime::MultiSigner: From<<TPublic::Pair as Pair>::Public>,
{
	sp_runtime::MultiSigner::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Canonical dev collator set for a fork genesis: Alice, Bob, Charlie.
///
/// Each collator is endowed with `100 * UNITS` of native token.  The AuraId
/// keys are derived from the same SR25519 dev seed so that a local dev node
/// can produce blocks without extra keystore setup.
///
/// # Stability guarantee
///
/// The order — Alice, Bob, Charlie — is stable across calls and must remain so;
/// the `CollatorSelection` pallet treats the first entry in `invulnerables` as
/// the preferred block author when multiple options are valid.
pub fn dev_collator_set() -> Vec<CollatorTuple> {
	const COLLATOR_ENDOWMENT: Balance = 100 * UNITS;
	vec![
		(
			account_id_from_seed::<sp_core::sr25519::Public>("Alice"),
			COLLATOR_ENDOWMENT,
			get_from_seed::<AuraId>("Alice"),
		),
		(
			account_id_from_seed::<sp_core::sr25519::Public>("Bob"),
			COLLATOR_ENDOWMENT,
			get_from_seed::<AuraId>("Bob"),
		),
		(
			account_id_from_seed::<sp_core::sr25519::Public>("Charlie"),
			COLLATOR_ENDOWMENT,
			get_from_seed::<AuraId>("Charlie"),
		),
	]
}

/// Assemble a `general_runtime::GenesisConfig` suitable for a fork-based chain
/// spec, given an explicit WASM binary and collator set.
///
/// # Parameters
///
/// - `wasm_binary` — the runtime WASM blob to embed in genesis.  Fork specs
///   always supply the new runtime explicitly; this avoids relying on the
///   compile-time `WASM_BINARY` constant which may not match the forked state.
/// - `collators` — `Vec<CollatorTuple>` (account, endowment, aura key).
///   Accounts are added to `BalancesConfig` using their stated endowment.
///   `CollatorSelectionConfig.invulnerables` and `SessionConfig.keys` are
///   derived from the same list.
/// - `root_key` — installed as the `SudoConfig.key`; `None` disables sudo.
/// - `para_id` — the parachain ID, stored in `ParachainInfoConfig`.
/// - `extra_endowed` — **additive** extra balance grants.  These accounts are
///   appended to the balance list after collator endowments.  If an account
///   appears in both `collators` and `extra_endowed`, its final on-chain
///   balance is the **sum** of both endowments (Balances pallet sums duplicate
///   entries at genesis).  The choice is deliberately additive — fork specs
///   often top-up treasury or test accounts without wanting to override
///   collator balances.
///
/// # Field correspondence
///
/// All 12 `GenesisConfig` fields are populated.  See `testnet_genesis` in
/// `node/src/chain_spec/testnet/mod.rs` for the canonical reference.
pub fn assemble_general_fork_genesis(
	wasm_binary: &[u8],
	collators: Vec<CollatorTuple>,
	root_key: Option<AccountId>,
	para_id: ParaId,
	extra_endowed: Vec<(AccountId, Balance)>,
) -> general_runtime::GenesisConfig {
	// Build the combined balance list: collator endowments first, then extras.
	// The Balances pallet sums duplicate AccountId entries at genesis — this is
	// intentional and documented in the doc-comment above.
	let balances: Vec<(AccountId, Balance)> = collators
		.iter()
		.map(|(acc, bal, _)| (acc.clone(), *bal))
		.chain(extra_endowed.into_iter())
		.collect();

	general_runtime::GenesisConfig {
		// 1. system
		system: general_runtime::SystemConfig { code: wasm_binary.to_vec() },
		// 2. balances
		balances: general_runtime::BalancesConfig { balances },
		// 3. parachain_info
		parachain_info: general_runtime::ParachainInfoConfig { parachain_id: para_id },
		// 4. collator_selection
		collator_selection: general_runtime::CollatorSelectionConfig {
			invulnerables: collators.iter().map(|(acc, _, _)| acc.clone()).collect(),
			candidacy_bond: 100 * UNITS,
			..Default::default()
		},
		// 5. session
		session: general_runtime::SessionConfig {
			keys: collators
				.into_iter()
				.map(|(acc, _, aura)| {
					(
						acc.clone(),
						acc,
						general_runtime::SessionKeys { aura },
					)
				})
				.collect(),
		},
		// 6. aura — intentionally default; Session populates Aura's authority set.
		aura: Default::default(),
		// 7. aura_ext
		aura_ext: Default::default(),
		// 8. parachain_system
		parachain_system: Default::default(),
		// 9. polkadot_xcm
		polkadot_xcm: general_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(super::SAFE_XCM_VERSION),
		},
		// 10. transaction_payment
		transaction_payment: Default::default(),
		// 11. assets
		assets: Default::default(),
		// 12. sudo
		sudo: general_runtime::SudoConfig { key: root_key },
	}
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Returns `true` if `key` matches any drop rule.
#[inline]
fn should_drop(key: &[u8], pallet_prefixes: &[[u8; 16]], item_prefixes: &[[u8; 32]]) -> bool {
	// (c) exact well-known keys — checked first; short keys with early-exit on
	//     length mismatch make this O(1) per entry.
	if LEAFCHAIN_DROP_EXACT.iter().any(|exact| key == *exact) {
		return true
	}

	// (a) pallet-level prefix (16 bytes)
	if key.len() >= 16 && pallet_prefixes.iter().any(|p| key.starts_with(p)) {
		return true
	}

	// (b) item-level prefix (32 bytes)
	if key.len() >= 32 && item_prefixes.iter().any(|p| key.starts_with(p)) {
		return true
	}

	false
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::storage::{Storage, StorageChild};

	/// Build a `Storage` with only a `top` map; `children_default` is empty.
	fn storage_with_keys(keys: &[Vec<u8>]) -> Storage {
		let top = keys.iter().map(|k| (k.clone(), vec![0xdeu8, 0xad])).collect();
		Storage { top, children_default: Default::default() }
	}

	/// 16-byte pallet prefix helper (mirrors production code).
	fn pallet_prefix(name: &[u8]) -> Vec<u8> {
		twox_128(name).to_vec()
	}

	/// 32-byte item prefix helper.
	fn item_prefix(pallet: &[u8], item: &[u8]) -> Vec<u8> {
		let mut v = Vec::with_capacity(32);
		v.extend_from_slice(&twox_128(pallet));
		v.extend_from_slice(&twox_128(item));
		v
	}

	/// A synthetic key with the given prefix followed by `suffix`.
	fn keyed(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
		let mut k = prefix.to_vec();
		k.extend_from_slice(suffix);
		k
	}

	// -----------------------------------------------------------------------
	// Test 1: Aura pallet — whole pallet dropped
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_aura_whole_pallet() {
		let aura_key = keyed(&pallet_prefix(b"Aura"), b"CurrentSlot_suffix");
		let input = storage_with_keys(&[aura_key.clone()]);
		let output = filter_forked_storage(input);
		assert!(!output.top.contains_key(&aura_key), "Aura pallet key must be dropped");
	}

	// -----------------------------------------------------------------------
	// Test 2: CollatorSelection pallet — whole pallet dropped
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_collator_selection_whole_pallet() {
		let cs_key = keyed(&pallet_prefix(b"CollatorSelection"), b"Candidates_suffix");
		let input = storage_with_keys(&[cs_key.clone()]);
		let output = filter_forked_storage(input);
		assert!(!output.top.contains_key(&cs_key), "CollatorSelection pallet key must be dropped");
	}

	// -----------------------------------------------------------------------
	// Test 3: ParachainSystem.LastRelayChainBlockNumber — item-level drop
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_parachain_system_last_relay_chain_block_number() {
		let key = item_prefix(b"ParachainSystem", b"LastRelayChainBlockNumber");
		// item-level keys are exactly 32 bytes (no map suffix needed for this test)
		let input = storage_with_keys(&[key.clone()]);
		let output = filter_forked_storage(input);
		assert!(
			!output.top.contains_key(&key),
			"ParachainSystem.LastRelayChainBlockNumber must be dropped"
		);
	}

	// -----------------------------------------------------------------------
	// Test 4: ParachainSystem.ValidationData — item-level drop
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_parachain_system_validation_data() {
		// Build a key with the 32-byte prefix plus extra bytes (as it would appear
		// in a real snapshot — storage value keys may be longer than 32 bytes).
		let key = keyed(&item_prefix(b"ParachainSystem", b"ValidationData"), b"extra_suffix");
		let input = storage_with_keys(&[key.clone()]);
		let output = filter_forked_storage(input);
		assert!(
			!output.top.contains_key(&key),
			"ParachainSystem.ValidationData must be dropped"
		);
	}

	// -----------------------------------------------------------------------
	// Test 5: ParachainInfo.ParachainId — MUST be preserved (critical invariant)
	// -----------------------------------------------------------------------
	#[test]
	fn filter_preserves_parachain_info_parachain_id() {
		// ParachainInfo is NOT in LEAFCHAIN_DROP_PALLETS.
		// ParachainInfo.ParachainId is NOT in LEAFCHAIN_DROP_ITEMS.
		// This is the single most critical preserve invariant for a parachain fork.
		let key = keyed(&item_prefix(b"ParachainInfo", b"ParachainId"), b"");
		let input = storage_with_keys(&[key.clone()]);
		let output = filter_forked_storage(input);
		assert!(
			output.top.contains_key(&key),
			"ParachainInfo.ParachainId must be preserved — dropping it would break parachain registration"
		);
	}

	// -----------------------------------------------------------------------
	// Test 6: System.Account — MUST be preserved (account balances)
	// -----------------------------------------------------------------------
	#[test]
	fn filter_preserves_system_account() {
		// System is not in DROP_PALLETS.
		// System.Account is not in DROP_ITEMS.
		// A 48-byte key: 32-byte prefix + 16-byte account suffix (Blake2_128Concat).
		let account_key = keyed(&item_prefix(b"System", b"Account"), &[0xabu8; 16]);
		let input = storage_with_keys(&[account_key.clone()]);
		let output = filter_forked_storage(input);
		assert!(
			output.top.contains_key(&account_key),
			"System.Account must be preserved — account balances must survive the fork"
		);
	}

	// -----------------------------------------------------------------------
	// Test 7: :extrinsic_index — exact-match drop
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_extrinsic_index_exact() {
		let key = b":extrinsic_index".to_vec();
		let input = storage_with_keys(&[key.clone()]);
		let output = filter_forked_storage(input);
		assert!(!output.top.contains_key(&key), ":extrinsic_index must be dropped");
	}

	// -----------------------------------------------------------------------
	// Test 8: :code and :heappages preserved; :intrablock_entropy dropped
	// -----------------------------------------------------------------------
	#[test]
	fn filter_preserves_code_and_heappages() {
		let code_key = b":code".to_vec();
		let heappages_key = b":heappages".to_vec();
		let entropy_key = b":intrablock_entropy".to_vec();

		let input =
			storage_with_keys(&[code_key.clone(), heappages_key.clone(), entropy_key.clone()]);
		let output = filter_forked_storage(input);

		assert!(output.top.contains_key(&code_key), ":code must be preserved");
		assert!(output.top.contains_key(&heappages_key), ":heappages must be preserved");
		assert!(!output.top.contains_key(&entropy_key), ":intrablock_entropy must be dropped");
	}

	// -----------------------------------------------------------------------
	// Bonus: children_default passes through byte-identical
	// -----------------------------------------------------------------------
	#[test]
	fn filter_children_default_passes_through_unchanged() {
		let child_key = b"parachain-child-root".to_vec();
		let child_value = StorageChild {
			data: vec![(b"inner".to_vec(), b"value".to_vec())].into_iter().collect(),
			child_info: sp_core::storage::ChildInfo::new_default(b"parachain-child-root"),
		};

		let mut input = Storage::default();
		let _ = input.children_default.insert(child_key.clone(), child_value.clone());
		// Add a top-level key that will be dropped to confirm children are unaffected.
		let _ = input.top.insert(b":extrinsic_index".to_vec(), b"transient".to_vec());

		let output = filter_forked_storage(input);

		assert!(
			output.children_default.contains_key(&child_key),
			"children_default entry must survive filter"
		);
		assert_eq!(
			output.children_default[&child_key].data,
			child_value.data,
			"children_default data must be byte-identical"
		);
		assert!(
			!output.top.contains_key(b":extrinsic_index".as_ref()),
			":extrinsic_index must still be dropped"
		);
	}

	// -----------------------------------------------------------------------
	// Bonus: All 8 DROP_PALLETS are actually removed
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_all_leafchain_drop_pallets() {
		let keys: Vec<Vec<u8>> = LEAFCHAIN_DROP_PALLETS
			.iter()
			.map(|name| keyed(&pallet_prefix(name), b"_any_storage_item_suffix"))
			.collect();

		let input = storage_with_keys(&keys);
		let output = filter_forked_storage(input);

		for (name, key) in LEAFCHAIN_DROP_PALLETS.iter().zip(keys.iter()) {
			assert!(
				!output.top.contains_key(key),
				"Pallet {:?} key must be dropped",
				core::str::from_utf8(name).unwrap_or("<non-utf8>")
			);
		}
	}

	// -----------------------------------------------------------------------
	// Test T2-1: dev_collator_set returns 3 distinct AccountIds
	// -----------------------------------------------------------------------
	#[test]
	fn dev_collator_set_has_three_distinct_accounts() {
		let collators = dev_collator_set();
		assert_eq!(collators.len(), 3, "dev_collator_set must return exactly 3 entries");

		let account_ids: Vec<AccountId> = collators.iter().map(|(acc, _, _)| acc.clone()).collect();

		// All three AccountIds must be pairwise distinct.
		assert_ne!(account_ids[0], account_ids[1], "Alice and Bob must have distinct AccountIds");
		assert_ne!(account_ids[1], account_ids[2], "Bob and Charlie must have distinct AccountIds");
		assert_ne!(account_ids[0], account_ids[2], "Alice and Charlie must have distinct AccountIds");

		// Each AuraId must also be distinct (derived from different sr25519 keys).
		let aura_ids: Vec<AuraId> = collators.iter().map(|(_, _, aura)| aura.clone()).collect();
		assert_ne!(aura_ids[0], aura_ids[1], "Alice and Bob must have distinct AuraIds");
		assert_ne!(aura_ids[1], aura_ids[2], "Bob and Charlie must have distinct AuraIds");
		assert_ne!(aura_ids[0], aura_ids[2], "Alice and Charlie must have distinct AuraIds");
	}

	// -----------------------------------------------------------------------
	// Test T2-2: assemble_general_fork_genesis populates all 12 fields without
	// panic and produces structurally correct output
	// -----------------------------------------------------------------------
	#[test]
	fn assemble_general_fork_genesis_populates_all_12_fields_without_panic() {
		use cumulus_primitives_core::ParaId;

		let wasm = vec![0x00u8, 0x61, 0x73, 0x6d]; // minimal wasm magic bytes
		let collators = dev_collator_set();
		let root_key = account_id_from_seed::<sp_core::sr25519::Public>("Alice");
		let para_id = ParaId::from(2000u32);

		// No extra endowed accounts in the baseline test.
		let genesis =
			assemble_general_fork_genesis(&wasm, collators.clone(), Some(root_key.clone()), para_id, vec![]);

		// Field 1 (system): wasm blob is stored verbatim.
		assert_eq!(genesis.system.code, wasm, "system.code must store the supplied wasm binary");

		// Field 2 (balances): one entry per collator.
		assert_eq!(
			genesis.balances.balances.len(),
			3,
			"balances must have one entry per collator when extra_endowed is empty"
		);

		// Field 3 (parachain_info): para_id preserved.
		assert_eq!(
			genesis.parachain_info.parachain_id,
			para_id,
			"parachain_info.parachain_id must match supplied para_id"
		);

		// Field 4 (collator_selection): invulnerables has 3 entries.
		assert_eq!(
			genesis.collator_selection.invulnerables.len(),
			3,
			"collator_selection.invulnerables must have one entry per collator"
		);

		// Field 5 (session): 3 key entries.
		assert_eq!(
			genesis.session.keys.len(),
			3,
			"session.keys must have one entry per collator"
		);

		// Field 12 (sudo): root_key is present.
		assert_eq!(
			genesis.sudo.key,
			Some(root_key),
			"sudo.key must match the supplied root_key"
		);
	}

	// -----------------------------------------------------------------------
	// Test T2-3: extra_endowed is additive — collator balances are not
	// overwritten; the extra accounts appear in addition to collator entries.
	// Design choice: additive, not overwriting (documented in function doc).
	// -----------------------------------------------------------------------
	#[test]
	fn assemble_general_fork_genesis_extra_endowed_is_additive() {
		use cumulus_primitives_core::ParaId;

		let wasm = vec![0x00u8, 0x61, 0x73, 0x6d];
		let collators = dev_collator_set();
		let root_key = account_id_from_seed::<sp_core::sr25519::Public>("Alice");
		let para_id = ParaId::from(2000u32);

		// Dave is an extra account not in the collator set.
		let dave = account_id_from_seed::<sp_core::sr25519::Public>("Dave");
		let extra: Vec<(AccountId, Balance)> = vec![(dave.clone(), 500 * UNITS)];

		let genesis = assemble_general_fork_genesis(
			&wasm,
			collators.clone(),
			Some(root_key),
			para_id,
			extra,
		);

		// Total balance entries: 3 collators + 1 extra = 4.
		assert_eq!(
			genesis.balances.balances.len(),
			4,
			"extra_endowed must be appended (additive), not merged or deduplicated"
		);

		// The collator entries must still be present at their original endowment.
		let alice_acc = account_id_from_seed::<sp_core::sr25519::Public>("Alice");
		let alice_entry =
			genesis.balances.balances.iter().find(|(acc, _)| acc == &alice_acc);
		assert!(alice_entry.is_some(), "Alice's balance entry must survive extra_endowed addition");
		assert_eq!(
			alice_entry.unwrap().1,
			100 * UNITS,
			"Alice's collator endowment must remain 100 * UNITS"
		);

		// Dave's extra entry must be present.
		let dave_entry =
			genesis.balances.balances.iter().find(|(acc, _)| acc == &dave);
		assert!(dave_entry.is_some(), "Dave's extra_endowed entry must be present in balances");
		assert_eq!(
			dave_entry.unwrap().1,
			500 * UNITS,
			"Dave's balance must be exactly the extra_endowed amount"
		);
	}

	// -----------------------------------------------------------------------
	// Test T2-4: root_key is reflected in sudo field
	// Verifies the sudo field receives the exact AccountId passed as root_key,
	// and that passing None disables sudo (None → no sudo key).
	// -----------------------------------------------------------------------
	#[test]
	fn assemble_general_fork_genesis_root_key_reflected_in_sudo() {
		use cumulus_primitives_core::ParaId;

		let wasm = vec![0x00u8, 0x61, 0x73, 0x6d];
		let collators = dev_collator_set();
		let para_id = ParaId::from(2000u32);

		// Case A: Some(root_key) → sudo.key is Some.
		let root_key = account_id_from_seed::<sp_core::sr25519::Public>("Eve");
		let genesis_with_sudo = assemble_general_fork_genesis(
			&wasm,
			collators.clone(),
			Some(root_key.clone()),
			para_id,
			vec![],
		);
		assert_eq!(
			genesis_with_sudo.sudo.key,
			Some(root_key),
			"sudo.key must be Some(root_key) when root_key is supplied"
		);

		// Case B: None → sudo.key is None (sudo disabled).
		let genesis_no_sudo =
			assemble_general_fork_genesis(&wasm, collators, None, para_id, vec![]);
		assert_eq!(
			genesis_no_sudo.sudo.key,
			None,
			"sudo.key must be None when root_key is None"
		);
	}

	// -----------------------------------------------------------------------
	// Test T3-1: DROP tables are sorted and deduplicated (strict < invariant)
	// -----------------------------------------------------------------------
	#[test]
	fn drop_lists_are_deduplicated_and_sorted() {
		// LEAFCHAIN_DROP_PALLETS: each consecutive pair must satisfy strict <.
		assert!(
			LEAFCHAIN_DROP_PALLETS.windows(2).all(|w| w[0] < w[1]),
			"LEAFCHAIN_DROP_PALLETS must be strictly sorted (no duplicates, no mis-ordering)"
		);

		// LEAFCHAIN_DROP_ITEMS: tuples compared lexicographically (pallet, item).
		// The Ord impl on (&[u8], &[u8]) is lexicographic, which is exactly what we want.
		assert!(
			LEAFCHAIN_DROP_ITEMS.windows(2).all(|w| w[0] < w[1]),
			"LEAFCHAIN_DROP_ITEMS must be strictly sorted by (pallet, item) — no duplicates, no mis-ordering"
		);

		// LEAFCHAIN_DROP_EXACT: bare keys compared byte-by-byte.
		assert!(
			LEAFCHAIN_DROP_EXACT.windows(2).all(|w| w[0] < w[1]),
			"LEAFCHAIN_DROP_EXACT must be strictly sorted (no duplicates, no mis-ordering)"
		);
	}

	// -----------------------------------------------------------------------
	// Test T3-2: Synthetic merge — filter + assemble produces no collision
	// -----------------------------------------------------------------------
	#[test]
	fn filter_and_assemble_merge_produces_no_collision() {
		use codec::Encode;
		use cumulus_primitives_core::ParaId;

		// --- Build synthetic Storage with three representative keys ---

		// KEEP key: ParachainInfo.ParachainId → SCALE-encoded 7u32.
		let keep_key = item_prefix(b"ParachainInfo", b"ParachainId");
		let keep_val = 7u32.encode();

		// DROP key: Aura.Authorities → a dummy value.
		let drop_key = item_prefix(b"Aura", b"Authorities");
		let drop_val = vec![0xAAu8];

		// Well-known key: :code → minimal wasm magic bytes.
		let code_key = b":code".to_vec();
		let code_val = vec![0x00u8, 0x61, 0x73, 0x6d];

		let mut raw_top = std::collections::BTreeMap::new();
		let _ = raw_top.insert(keep_key.clone(), keep_val);
		let _ = raw_top.insert(drop_key.clone(), drop_val);
		let _ = raw_top.insert(code_key.clone(), code_val.clone());

		let storage = sp_core::storage::Storage {
			top: raw_top,
			children_default: Default::default(),
		};

		// --- Apply filter ---
		let filtered = filter_forked_storage(storage);

		// (a) Aura.Authorities must NOT survive the filter.
		assert!(
			!filtered.top.contains_key(&drop_key),
			"Aura.Authorities must be dropped by filter_forked_storage"
		);

		// (b) ParachainInfo.ParachainId must survive the filter.
		assert!(
			filtered.top.contains_key(&keep_key),
			"ParachainInfo.ParachainId must be preserved by filter_forked_storage"
		);

		// (c) :code must survive the filter.
		assert!(
			filtered.top.contains_key(&code_key),
			":code must be preserved by filter_forked_storage"
		);

		// --- Extract wasm and call assemble ---
		let wasm = filtered.top.get(&code_key).unwrap().clone();
		assert_eq!(wasm, code_val, ":code value must be byte-identical after filter");

		let para_id = ParaId::from(7u32);
		let alice = account_id_from_seed::<sp_core::sr25519::Public>("Alice");
		let collators = dev_collator_set();

		let assembled = assemble_general_fork_genesis(
			&wasm,
			collators,
			Some(alice),
			para_id,
			vec![],
		);

		// Assert assembled.parachain_info.parachain_id == ParaId::from(7).
		assert_eq!(
			assembled.parachain_info.parachain_id,
			para_id,
			"assemble_general_fork_genesis must stamp ParaId::from(7) into parachain_info"
		);
	}

	// -----------------------------------------------------------------------
	// Bonus: All 26 DROP_ITEMS are actually removed
	// -----------------------------------------------------------------------
	#[test]
	fn filter_drops_all_leafchain_drop_items() {
		let keys: Vec<Vec<u8>> = LEAFCHAIN_DROP_ITEMS
			.iter()
			.map(|(pallet, item)| item_prefix(pallet, item))
			.collect();

		let input = storage_with_keys(&keys);
		let output = filter_forked_storage(input);

		for ((pallet, item), key) in LEAFCHAIN_DROP_ITEMS.iter().zip(keys.iter()) {
			assert!(
				!output.top.contains_key(key),
				"Item ({:?}, {:?}) must be dropped",
				core::str::from_utf8(pallet).unwrap_or("<non-utf8>"),
				core::str::from_utf8(item).unwrap_or("<non-utf8>")
			);
		}
	}
}
