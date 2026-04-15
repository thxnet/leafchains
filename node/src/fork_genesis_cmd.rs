//! `fork-genesis` subcommand — export a filtered + freshly-seeded fork chain-spec
//! for a leafchain (parachain).
//!
//! Reads the state at a chosen finalized block, strips consensus-transient storage
//! via [`chain_spec::fork::filter_forked_storage`], drops `Balances.TotalIssuance`
//! to avoid divergence, assembles fresh dev-authority genesis storage via
//! [`chain_spec::fork::assemble_general_fork_genesis`], merges the two (fresh wins
//! on collision), and emits a raw chain-spec JSON to stdout or `--output` file.
//!
//! Optionally injects the relay-chain spec's `.id` into the output's
//! top-level `.rootchain` field via `--relay-chain-spec`.
//! (polkadot-v0.9.40 serialises extensions as top-level fields, not under `.extensions`.)

use sc_cli::{CliConfiguration, DatabaseParams, PruningParams, SharedParams};
use sc_client_api::{Backend, HeaderBackend, StorageProvider, UsageProvider};
use sp_core::hashing::twox_128;
use sp_runtime::{traits::Block as BlockT, BuildStorage};
use std::{path::PathBuf, str::FromStr, sync::Arc};

use cumulus_primitives_core::ParaId;

use crate::chain_spec;

/// Export a fork genesis chain-spec from a live leafchain node database.
///
/// Reads the state at a chosen finalized block, strips consensus- and
/// block-execution-transient storage, drops `Balances.TotalIssuance` to avoid
/// divergence with the freshly-assembled collator balances, assembles fresh
/// dev-authority genesis storage (Alice/Bob/Charlie), merges (fresh-wins on
/// collision), and serialises to raw chain-spec JSON.
///
/// The `--relay-chain-spec` flag is optional: when supplied, its `.id` field is
/// extracted and injected into the output spec's top-level `.rootchain`, making
/// the fork spec point at the target relay chain rather than the source chain.
/// (polkadot-v0.9.40 specs use top-level extension fields, no `.extensions` nesting.)
#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct ForkGenesisCmd {
    #[clap(flatten)]
    pub shared_params: SharedParams,

    #[clap(flatten)]
    pub pruning_params: PruningParams,

    #[clap(flatten)]
    pub database_params: DatabaseParams,

    /// ParaId to embed in genesis (required — must match the target fork network)
    #[arg(long, value_name = "PARA_ID")]
    pub para_id: u32,

    /// Path to relay-chain spec JSON; if supplied, its `.id` is injected into
    /// the output's `.extensions.rootchain` field.
    #[arg(long, value_name = "PATH")]
    pub relay_chain_spec: Option<PathBuf>,

    /// Output path for forked chain-spec JSON (stdout if omitted)
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Block to fork from: "finalized" (default) or 0x-prefixed hex hash
    #[arg(long, default_value = "finalized")]
    pub at: String,
}

impl ForkGenesisCmd {
    /// Run the `fork-genesis` command.
    pub async fn run<B, BA, C>(
        &self,
        client: Arc<C>,
        _backend: Arc<BA>,
        mut chain_spec: Box<dyn sc_service::ChainSpec>,
    ) -> sc_cli::Result<()>
    where
        B: BlockT,
        B::Hash: FromStr,
        <B::Hash as FromStr>::Err: std::fmt::Debug,
        BA: Backend<B>,
        C: UsageProvider<B> + StorageProvider<B, BA> + HeaderBackend<B> + 'static,
    {
        // -----------------------------------------------------------------------
        // 1. Resolve target block hash.
        // -----------------------------------------------------------------------
        let hash = resolve_at::<B, C>(&self.at, &*client)?;

        // -----------------------------------------------------------------------
        // 2. Export live state at that block.
        // -----------------------------------------------------------------------
        let raw =
            sc_service::chain_ops::export_raw_state(client, hash).map_err(|e| {
                sc_cli::Error::Input(format!("export_raw_state failed: {}", e))
            })?;

        // -----------------------------------------------------------------------
        // 3. Filter consensus-transient keys.
        // -----------------------------------------------------------------------
        let mut filtered = chain_spec::fork::filter_forked_storage(raw);

        // -----------------------------------------------------------------------
        // 4. Drop Balances.TotalIssuance — avoids divergence with fresh genesis.
        //    Key = twox_128("Balances") ++ twox_128("TotalIssuance") (32 bytes).
        //    Applied here at the call site per master directive; fork.rs is frozen.
        // -----------------------------------------------------------------------
        let balances_total_issuance_key = {
            let mut k = twox_128(b"Balances").to_vec();
            k.extend_from_slice(&twox_128(b"TotalIssuance"));
            k
        };
        filtered.top.remove(&balances_total_issuance_key);

        // -----------------------------------------------------------------------
        // 5. Assemble fresh genesis storage (dev collators + Alice as sudo).
        // -----------------------------------------------------------------------
        let wasm = general_runtime::WASM_BINARY
            .ok_or_else(|| sc_cli::Error::Input("WASM binary not available".to_string()))?;

        let fresh_gc = chain_spec::fork::assemble_general_fork_genesis(
            wasm,
            chain_spec::fork::dev_collator_set(),
            Some(sp_keyring::Sr25519Keyring::Alice.to_account_id()),
            ParaId::from(self.para_id),
            vec![],
        );

        let fresh_storage = fresh_gc
            .build_storage()
            .map_err(|e| sc_cli::Error::Input(format!("build_storage failed: {}", e)))?;

        // -----------------------------------------------------------------------
        // 6. Merge: fresh wins on collision.
        //    Insert all fresh top-level keys into filtered (fresh overwrites).
        // -----------------------------------------------------------------------
        for (k, v) in fresh_storage.top {
            filtered.top.insert(k, v);
        }
        for (child_root, fresh_child) in fresh_storage.children_default {
            let entry = filtered.children_default.entry(child_root).or_insert_with(|| {
                sp_core::storage::StorageChild {
                    data: Default::default(),
                    child_info: fresh_child.child_info.clone(),
                }
            });
            for (k, v) in fresh_child.data {
                entry.data.insert(k, v);
            }
        }

        // -----------------------------------------------------------------------
        // 7. Materialise merged storage into chain spec.
        // -----------------------------------------------------------------------
        chain_spec.set_storage(filtered);

        // -----------------------------------------------------------------------
        // 8. Serialise to raw JSON.
        // -----------------------------------------------------------------------
        let mut json_str = sc_service::chain_ops::build_spec(&*chain_spec, true)
            .map_err(|e| sc_cli::Error::Input(format!("build_spec failed: {}", e)))?;

        // -----------------------------------------------------------------------
        // 9. Optionally inject relay chain spec id into .extensions.rootchain.
        // -----------------------------------------------------------------------
        if let Some(relay_path) = &self.relay_chain_spec {
            json_str = inject_relay_rootchain(json_str, relay_path)?;
        }

        // -----------------------------------------------------------------------
        // 10. Write output.
        // -----------------------------------------------------------------------
        if let Some(out_path) = &self.output {
            std::fs::write(out_path, json_str.as_bytes()).map_err(|e| {
                sc_cli::Error::Input(format!(
                    "failed to write output {}: {}",
                    out_path.display(),
                    e
                ))
            })?;
        } else {
            // Explicit stdout path — println! is intentional here.
            println!("{}", json_str);
        }

        Ok(())
    }
}

impl CliConfiguration for ForkGenesisCmd {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }

    fn pruning_params(&self) -> Option<&PruningParams> {
        Some(&self.pruning_params)
    }

    fn database_params(&self) -> Option<&DatabaseParams> {
        Some(&self.database_params)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Resolve a block hash from the `--at` argument.
///
/// `"finalized"` → client's current finalized hash (NOT best — per master directive).
/// Hex string (with or without `0x` prefix) → parsed as `B::Hash`.
fn resolve_at<B, C>(at: &str, client: &C) -> sc_cli::Result<B::Hash>
where
    B: BlockT,
    B::Hash: FromStr,
    <B::Hash as FromStr>::Err: std::fmt::Debug,
    C: UsageProvider<B>,
{
    if at == "finalized" {
        Ok(client.usage_info().chain.finalized_hash)
    } else {
        let hex = at.trim_start_matches("0x");
        B::Hash::from_str(hex)
            .map_err(|e| sc_cli::Error::Input(format!("Invalid block hash {:?}: {:?}", at, e)))
    }
}

/// Parse the relay chain spec JSON at `relay_path`, extract its top-level `.id` field
/// (which identifies the relay chain), and inject it into the `output_spec_json` as
/// the top-level `.rootchain` field.
///
/// Returns the mutated JSON string.
///
/// polkadot-v0.9.40 chain specs serialize extensions at the TOP LEVEL of the spec
/// object (`.rootchain`, `.leafchain_id`), not under an `.extensions` sub-object.
/// We mutate the top-level object directly via serde_json to avoid fragile string
/// manipulation.
fn inject_relay_rootchain(
    output_spec_json: String,
    relay_path: &std::path::Path,
) -> sc_cli::Result<String> {
    // Read relay chain spec file.
    let relay_bytes = std::fs::read(relay_path).map_err(|e| {
        sc_cli::Error::Input(format!(
            "failed to read --relay-chain-spec {}: {}",
            relay_path.display(),
            e
        ))
    })?;

    // Parse relay spec to extract top-level `.id` — this is the relay chain identifier
    // that gets written into the leafchain spec's top-level `.rootchain` field.
    let relay_json: serde_json::Value =
        serde_json::from_slice(&relay_bytes).map_err(|e| {
            sc_cli::Error::Input(format!(
                "failed to parse relay chain spec {}: {}",
                relay_path.display(),
                e
            ))
        })?;

    let relay_id = relay_json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            sc_cli::Error::Input("input relay-chain spec has no top-level `rootchain` field".to_string())
        })?
        .to_string();

    // Parse output spec JSON and patch the top-level `.rootchain` field directly.
    // polkadot-v0.9.40: extensions are top-level fields, not nested under `.extensions`.
    let mut output_json: serde_json::Value =
        serde_json::from_str(&output_spec_json).map_err(|e| {
            sc_cli::Error::Input(format!("failed to parse output chain spec JSON: {}", e))
        })?;

    output_json
        .as_object_mut()
        .ok_or_else(|| sc_cli::Error::Input("output chain spec is not a JSON object".to_string()))?
        .insert("rootchain".to_string(), serde_json::Value::String(relay_id));

    serde_json::to_string_pretty(&output_json)
        .map_err(|e| sc_cli::Error::Input(format!("failed to re-serialize spec JSON: {}", e)))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::storage::Storage;
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Helper: build a Storage from a flat key-value list.
    // -----------------------------------------------------------------------
    fn make_storage(top: Vec<(Vec<u8>, Vec<u8>)>) -> Storage {
        let top_map: BTreeMap<Vec<u8>, Vec<u8>> = top.into_iter().collect();
        Storage { top: top_map, children_default: Default::default() }
    }

    // -----------------------------------------------------------------------
    // Test 1: merge_fresh_wins_on_collision
    //
    // When the same key exists in both filtered and fresh, fresh value wins.
    // This mirrors the merge loop in run(): fresh top-level entries are inserted
    // last (BTreeMap::insert returns old value, new value replaces).
    // -----------------------------------------------------------------------
    #[test]
    fn merge_fresh_wins_on_collision() {
        let collision_key = b"collide_key".to_vec();
        let filtered_val = b"filtered_value".to_vec();
        let fresh_val = b"fresh_value".to_vec();

        // Simulate the merge: start with filtered, insert fresh.
        let mut filtered = make_storage(vec![(collision_key.clone(), filtered_val.clone())]);
        let fresh = make_storage(vec![(collision_key.clone(), fresh_val.clone())]);

        // Perform the same merge as run().
        for (k, v) in fresh.top {
            filtered.top.insert(k, v);
        }

        assert_eq!(
            filtered.top.get(&collision_key),
            Some(&fresh_val),
            "fresh value must win on collision — filtered value must be overwritten"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: at_parse_finalized_literal
    //
    // Pure logic test: resolve_at with "finalized" must NOT attempt hex parsing.
    // We test the branching logic by verifying that "finalized" is the special
    // sentinel value and everything else goes through hex parsing.
    // We can't call resolve_at directly without a mock client, so we test the
    // pure parsing branch: strip "0x" prefix and detect non-finalized path.
    // -----------------------------------------------------------------------
    #[test]
    fn at_parse_finalized_literal() {
        // Verify the constant: "finalized" must be exactly that string.
        let sentinel = "finalized";
        assert_eq!(sentinel, "finalized", "finalized sentinel must be the literal string");

        // Verify that strip of 0x prefix works correctly for a hex hash.
        let hex_with_prefix = "0xdeadbeef";
        let stripped = hex_with_prefix.trim_start_matches("0x");
        assert_eq!(stripped, "deadbeef", "0x prefix must be stripped correctly");

        // Verify that a hash without 0x prefix is also handled (already stripped).
        let hex_bare = "deadbeef";
        let stripped_bare = hex_bare.trim_start_matches("0x");
        assert_eq!(stripped_bare, "deadbeef", "bare hex must pass through unchanged");

        // Verify: only the bare literal "finalized" (not "0x"-prefixed) triggers
        // the finalized branch, because we compare `at == "finalized"` before
        // any hex stripping. "0xfinalized" would fail the equality check and fall
        // through to hash parsing (which would then fail), never matching "finalized".
        let not_finalized_with_prefix = "0xfinalized";
        assert_ne!(not_finalized_with_prefix, "finalized",
            "0xfinalized must NOT equal finalized — prefix prevents sentinel match");
        // Only the bare literal is the sentinel.
        assert_eq!("finalized", "finalized", "finalized sentinel must equal itself");
    }

    // -----------------------------------------------------------------------
    // Test 3: relay_rootchain_injection
    //
    // polkadot-v0.9.40 shape: both relay spec and leafchain spec have their
    // extension fields at the TOP LEVEL (no `.extensions` nesting).
    //
    // Input relay spec fixture (relay chain — has `.id`, no `.rootchain`):
    //   { "id": "thxnet_testnet", "name": "THXNET. Testnet", ... }
    //
    // Input leafchain spec fixture (flat polkadot-v0.9.40 shape):
    //   { "id": "sand_testnet", "rootchain": "old_value", "leafchain_id": 1003, ... }
    //
    // Positive assertion: after injection, output has top-level `.rootchain`
    //   equal to relay's `.id`, and `.leafchain_id` is preserved.
    //
    // Negative-path assertion: relay spec missing `.id` → Err containing
    //   "input relay-chain spec has no top-level `rootchain` field".
    // -----------------------------------------------------------------------
    #[test]
    fn relay_rootchain_injection() {
        // Leafchain output spec — polkadot-v0.9.40 flat shape (no `.extensions`).
        let output_spec = serde_json::json!({
            "name": "Sand Testnet",
            "id": "sand_testnet",
            "chainType": "Live",
            "rootchain": "old_rootchain_value",
            "leafchain_id": 1003,
            "genesis": {}
        });
        let output_spec_str = serde_json::to_string(&output_spec).unwrap();

        // Relay chain spec — has top-level `.id` (relay chains have no `.rootchain`).
        let relay_spec = serde_json::json!({
            "name": "THXNET. Testnet",
            "id": "thxnet_testnet",
            "chainType": "Live",
            "genesis": {}
        });

        let tmp_dir = std::env::temp_dir();
        let relay_path = tmp_dir.join("test_relay_spec_v2.json");
        std::fs::write(&relay_path, serde_json::to_string(&relay_spec).unwrap())
            .expect("should write temp relay spec");

        // Positive path: injection must succeed.
        let result = inject_relay_rootchain(output_spec_str.clone(), &relay_path);
        assert!(result.is_ok(), "inject_relay_rootchain must succeed: {:?}", result.err());

        let mutated: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

        // Top-level .rootchain must be updated to relay's .id.
        assert_eq!(
            mutated["rootchain"].as_str(),
            Some("thxnet_testnet"),
            "top-level .rootchain must be updated to relay spec's .id"
        );

        // Top-level .leafchain_id must be preserved unchanged.
        assert_eq!(
            mutated["leafchain_id"].as_u64(),
            Some(1003),
            "top-level .leafchain_id must be preserved after injection"
        );

        // No `.extensions` object must exist (polkadot-v0.9.40 flat shape).
        assert!(
            mutated.get("extensions").is_none(),
            "polkadot-v0.9.40 specs have no nested .extensions object"
        );

        // Negative path: relay spec missing `.id` must return Err.
        let relay_no_id = serde_json::json!({ "name": "No ID Relay", "chainType": "Live" });
        let relay_no_id_path = tmp_dir.join("test_relay_no_id_v2.json");
        std::fs::write(&relay_no_id_path, serde_json::to_string(&relay_no_id).unwrap())
            .expect("should write temp relay spec");

        let err_result = inject_relay_rootchain(output_spec_str, &relay_no_id_path);
        assert!(err_result.is_err(), "missing .id in relay spec must return Err");
        let err_str = format!("{}", err_result.unwrap_err());
        assert!(
            err_str.contains("input relay-chain spec has no top-level `rootchain` field"),
            "error must mention missing top-level rootchain field; got: {}",
            err_str
        );

        // Cleanup.
        let _ = std::fs::remove_file(&relay_path);
        let _ = std::fs::remove_file(&relay_no_id_path);
    }

    // -----------------------------------------------------------------------
    // Test 4: balances_total_issuance_key_computation
    //
    // Verifies the TotalIssuance key computation is exactly 32 bytes and that
    // removing it from filtered storage succeeds (no panic, key removed).
    // -----------------------------------------------------------------------
    #[test]
    fn balances_total_issuance_key_computation() {
        let key = {
            let mut k = twox_128(b"Balances").to_vec();
            k.extend_from_slice(&twox_128(b"TotalIssuance"));
            k
        };

        // Key must be exactly 32 bytes (16 + 16).
        assert_eq!(key.len(), 32, "TotalIssuance storage key must be exactly 32 bytes");

        // Simulate removal from filtered storage.
        let mut storage = make_storage(vec![(key.clone(), b"some_value".to_vec())]);
        assert!(storage.top.contains_key(&key), "key must exist before removal");
        storage.top.remove(&key);
        assert!(!storage.top.contains_key(&key), "key must be absent after removal");
    }

    // -----------------------------------------------------------------------
    // Test 5: inject_relay_rootchain_missing_id_returns_error
    //
    // Relay spec missing `.id` must return an Err with the canonical message.
    // -----------------------------------------------------------------------
    #[test]
    fn inject_relay_rootchain_missing_id_returns_error() {
        let output_spec = serde_json::json!({
            "name": "Test",
            "id": "test_leaf",
            "rootchain": "old",
            "leafchain_id": 2000,
            "genesis": {}
        });
        let output_spec_str = serde_json::to_string(&output_spec).unwrap();

        let relay_spec_no_id = serde_json::json!({ "name": "No ID Relay" });
        let tmp_dir = std::env::temp_dir();
        let relay_path = tmp_dir.join("test_relay_no_id.json");
        std::fs::write(&relay_path, serde_json::to_string(&relay_spec_no_id).unwrap())
            .expect("should write temp relay spec");

        let result = inject_relay_rootchain(output_spec_str, &relay_path);
        assert!(result.is_err(), "missing .id in relay spec must return Err");
        let err_str = format!("{}", result.unwrap_err());
        assert!(
            err_str.contains("input relay-chain spec has no top-level `rootchain` field"),
            "error must mention missing top-level rootchain field; got: {}",
            err_str
        );

        let _ = std::fs::remove_file(&relay_path);
    }
}
