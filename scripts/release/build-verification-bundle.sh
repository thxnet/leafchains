#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
build-verification-bundle.sh — emit a scenario-specific cross-chain verification bundle

Usage:
  scripts/release/build-verification-bundle.sh w6-t3-verify [--output-dir=PATH]

Options:
  --output-dir=PATH           Output directory (default: dist/verification-bundles/<scenario_id>)
  --leafchain-bin=PATH        thxnet-leafchain binary to use
  --seed-db=PATH              Base path of the source live leafchain DB
  --database=BACKEND          Database backend for fork-genesis (default: auto)
  --relay-chain=ID            Relay chain id to inject into the forked spec
  --relay-chain-spec=PATH     Relay chain spec JSON to read the .id from
  --leafchain-binary-ref=REF  OCI ref to record in manifest.leafchain_binary.image
  --source-at=AT              Block selector passed to fork-genesis --at (default: finalized)
  --help                      Show this help

Environment overrides:
  LEAFCHAIN_BIN
  LEAFCHAIN_SEED_DB
  LEAFCHAIN_DATABASE
  RELAY_CHAIN
  RELAY_CHAIN_SPEC
  LEAFCHAIN_BINARY_REF
  SOURCE_AT

Currently supported scenarios:
  - w6-t3-verify
EOF
}

repo_root() {
  git rev-parse --show-toplevel
}

sha256_of() {
  sha256sum "$1" | awk '{print $1}'
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

scenario_id=""
output_dir=""
leafchain_bin="${LEAFCHAIN_BIN:-}"
seed_db="${LEAFCHAIN_SEED_DB:-}"
database_backend="${LEAFCHAIN_DATABASE:-auto}"
relay_chain="${RELAY_CHAIN:-}"
relay_chain_spec="${RELAY_CHAIN_SPEC:-}"
leafchain_binary_ref="${LEAFCHAIN_BINARY_REF:-}"
source_at="${SOURCE_AT:-finalized}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    --output-dir=*)
      output_dir="${1#*=}"
      ;;
    --leafchain-bin=*)
      leafchain_bin="${1#*=}"
      ;;
    --seed-db=*)
      seed_db="${1#*=}"
      ;;
    --database=*)
      database_backend="${1#*=}"
      ;;
    --relay-chain=*)
      relay_chain="${1#*=}"
      ;;
    --relay-chain-spec=*)
      relay_chain_spec="${1#*=}"
      ;;
    --leafchain-binary-ref=*)
      leafchain_binary_ref="${1#*=}"
      ;;
    --source-at=*)
      source_at="${1#*=}"
      ;;
    --*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
    *)
      if [[ -z "$scenario_id" ]]; then
        scenario_id="$1"
      else
        echo "unexpected extra argument: $1" >&2
        usage >&2
        exit 1
      fi
      ;;
  esac
  shift
done

[[ -n "$scenario_id" ]] || {
  echo "missing scenario id" >&2
  usage >&2
  exit 1
}

root="$(repo_root)"
cd "$root"

require_cmd git
require_cmd python3
require_cmd sha256sum
require_cmd mktemp

case "$scenario_id" in
  w6-t3-verify)
    scenario_purpose="verify-cross-chain"
    leafchain_name="Sandbox"
    chain_name="sand-testnet"
    para_chain_id="sand_testnet"
    para_id="1003"
    relay_chain="${relay_chain:-thxnet-testnet}"
    seed_db="${seed_db:-/data/forknet-test/leafchain-sand-seed}"
    ;;
  *)
    echo "unsupported scenario: $scenario_id" >&2
    exit 1
    ;;
esac

output_dir="${output_dir:-$root/dist/verification-bundles/$scenario_id}"
leafchain_bin="${leafchain_bin:-$root/target/release/thxnet-leafchain}"

[[ -x "$leafchain_bin" ]] || {
  echo "leafchain binary is missing or not executable: $leafchain_bin" >&2
  echo "hint: build a release binary with runtime WASM available before running this producer" >&2
  exit 1
}
[[ -d "$seed_db" ]] || {
  echo "seed DB directory is missing: $seed_db" >&2
  exit 1
}

mkdir -p "$output_dir"
work_dir="$(mktemp -d "$output_dir/.tmp.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

if [[ -n "$leafchain_binary_ref" ]]; then
  require_cmd docker
  image_binary_path="$work_dir/leafchain-binary-from-image"
  image_container_name="verification-bundle-binary-$$"
  docker image inspect "$leafchain_binary_ref" >/dev/null 2>&1 || docker pull "$leafchain_binary_ref" >/dev/null
  docker rm -f "$image_container_name" >/dev/null 2>&1 || true
  docker create --name "$image_container_name" "$leafchain_binary_ref" >/dev/null
  docker cp "$image_container_name:/usr/local/bin/thxnet-leafchain" "$image_binary_path"
  docker rm -f "$image_container_name" >/dev/null
  chmod +x "$image_binary_path"

  local_binary_sha256="$(sha256_of "$leafchain_bin")"
  image_binary_sha256="$(sha256_of "$image_binary_path")"
  local_binary_version="$($leafchain_bin --version | head -n1 | tr -d '\r')"
  image_binary_version="$($image_binary_path --version | head -n1 | tr -d '\r')"

  if [[ "$local_binary_sha256" != "$image_binary_sha256" ]]; then
    echo "leafchain binary ref does not match the producer binary actually used" >&2
    echo "  producer binary: $leafchain_bin" >&2
    echo "  producer sha256: $local_binary_sha256" >&2
    echo "  producer version: $local_binary_version" >&2
    echo "  image ref: $leafchain_binary_ref" >&2
    echo "  image sha256: $image_binary_sha256" >&2
    echo "  image version: $image_binary_version" >&2
    echo "Either use a matching binary/image pair, or omit --leafchain-binary-ref for local unpublished bundles." >&2
    exit 1
  fi
fi

manifest_path="$output_dir/manifest.json"
para_spec_path="$output_dir/para-spec.raw.json"
validation_code_path="$output_dir/validation-code.wasm"
genesis_state_path="$output_dir/genesis-state.bin"
notes_path="$output_dir/notes.md"

relay_spec_source=""
if [[ -n "$relay_chain_spec" ]]; then
  [[ -f "$relay_chain_spec" ]] || {
    echo "relay chain spec file is missing: $relay_chain_spec" >&2
    exit 1
  }
  relay_spec_source="$relay_chain_spec"
else
  relay_spec_source="$work_dir/relay-chain-spec.json"
  python3 - "$relay_chain" "$relay_spec_source" <<'PY'
import json, sys
relay_chain, path = sys.argv[1:3]
with open(path, 'w', encoding='utf-8') as fh:
    json.dump({"id": relay_chain}, fh, indent=2, sort_keys=True)
    fh.write("\n")
PY
fi

tmp_para_spec="$work_dir/para-spec.raw.json"
tmp_genesis_state="$work_dir/genesis-state.bin"
tmp_validation_code="$work_dir/validation-code.wasm"

"$leafchain_bin" fork-genesis \
  --log=error \
  --database="$database_backend" \
  --base-path="$seed_db" \
  --chain="$chain_name" \
  --para-id="$para_id" \
  --relay-chain-spec="$relay_spec_source" \
  --at="$source_at" \
  --output="$tmp_para_spec"

"$leafchain_bin" export-genesis-state \
  --log=error \
  --raw \
  --chain="$tmp_para_spec" \
  "$tmp_genesis_state"

"$leafchain_bin" export-genesis-wasm \
  --log=error \
  --raw \
  --chain="$tmp_para_spec" \
  "$tmp_validation_code"

mv "$tmp_para_spec" "$para_spec_path"
mv "$tmp_genesis_state" "$genesis_state_path"
mv "$tmp_validation_code" "$validation_code_path"

leafchains_git_sha="$(git rev-parse HEAD)"
leafchain_version="$($leafchain_bin --version | head -n1 | tr -d '\r')"
leafchain_binary_sha256="$(sha256_of "$leafchain_bin")"
para_spec_sha256="$(sha256_of "$para_spec_path")"
validation_code_sha256="$(sha256_of "$validation_code_path")"
genesis_state_sha256="$(sha256_of "$genesis_state_path")"
generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
invocation="scripts/release/build-verification-bundle.sh $scenario_id"
para_spec_bytes="$(wc -c < "$para_spec_path" | tr -d ' ')"
validation_code_bytes="$(wc -c < "$validation_code_path" | tr -d ' ')"
genesis_state_bytes="$(wc -c < "$genesis_state_path" | tr -d ' ')"

python3 - \
  "$manifest_path" \
  "$scenario_id" \
  "$scenario_purpose" \
  "$relay_chain" \
  "$para_id" \
  "$para_chain_id" \
  "$leafchain_name" \
  "$leafchains_git_sha" \
  "$leafchain_version" \
  "$leafchain_binary_ref" \
  "$leafchain_binary_sha256" \
  "$seed_db" \
  "$database_backend" \
  "$source_at" \
  "$generated_at" \
  "$para_spec_sha256" \
  "$validation_code_sha256" \
  "$genesis_state_sha256" \
  "$para_spec_bytes" \
  "$validation_code_bytes" \
  "$genesis_state_bytes" <<'PY'
import json, sys
(
    manifest_path,
    scenario_id,
    scenario_purpose,
    relay_chain,
    para_id,
    para_chain_id,
    leafchain_name,
    leafchains_git_sha,
    leafchain_version,
    leafchain_binary_ref,
    leafchain_binary_sha256,
    seed_db,
    database_backend,
    source_at,
    generated_at,
    para_spec_sha256,
    validation_code_sha256,
    genesis_state_sha256,
    para_spec_bytes,
    validation_code_bytes,
    genesis_state_bytes,
) = sys.argv[1:]
manifest = {
    "schema_version": 1,
    "artifact_kind": "leafchain-cross-chain-verification-bundle",
    "scenario_id": scenario_id,
    "scenario_purpose": scenario_purpose,
    "relay_chain": relay_chain,
    "para_id": int(para_id),
    "para_chain_id": para_chain_id,
    "leafchain_name": leafchain_name,
    "leafchains_git_sha": leafchains_git_sha,
    "generated_at_utc": generated_at,
    "source": {
        "seed_db": seed_db,
        "database_backend": database_backend,
        "fork_at": source_at,
    },
    "leafchain_binary": {
        "image": leafchain_binary_ref or None,
        "version": leafchain_version,
        "sha256": leafchain_binary_sha256,
    },
    "paths": {
        "para_spec": "./para-spec.raw.json",
        "validation_code": "./validation-code.wasm",
        "genesis_state": "./genesis-state.bin",
    },
    "sizes": {
        "para_spec_bytes": int(para_spec_bytes),
        "validation_code_bytes": int(validation_code_bytes),
        "genesis_state_bytes": int(genesis_state_bytes),
    },
    "checksums": {
        "para_spec_sha256": para_spec_sha256,
        "validation_code_sha256": validation_code_sha256,
        "genesis_state_sha256": genesis_state_sha256,
    },
    "rootchain_contract": {
        "register_leafchain_mode": "spec-json",
        "verify_cross_chain_min_version": 1,
    },
}
with open(manifest_path, 'w', encoding='utf-8') as fh:
    json.dump(manifest, fh, indent=2, sort_keys=True)
    fh.write('\n')
PY

cat > "$notes_path" <<EOF
# $scenario_id verification bundle

Generated by:

- script: \
  scripts/release/build-verification-bundle.sh
- command: \
  $invocation
- time: \
  $generated_at

## Producer inputs

- leafchains git sha: \
  $leafchains_git_sha
- leafchain binary: \
  $leafchain_bin
- leafchain version: \
  $leafchain_version
- leafchain binary sha256: \
  $leafchain_binary_sha256
- source seed DB: \
  $seed_db
- database backend: \
  $database_backend
- fork source block selector: \
  $source_at
- relay chain id: \
  $relay_chain
- relay chain spec source: \
  $relay_spec_source

## Outputs

- manifest: \
  $manifest_path
- para spec: \
  $para_spec_path ($para_spec_bytes bytes)
- validation code: \
  $validation_code_path ($validation_code_bytes bytes)
- genesis state: \
  $genesis_state_path ($genesis_state_bytes bytes)

## Checksums

- para-spec.raw.json: \
  $para_spec_sha256
- validation-code.wasm: \
  $validation_code_sha256
- genesis-state.bin: \
  $genesis_state_sha256
EOF

printf 'verification bundle ready: %s\n' "$output_dir"
printf 'manifest: %s\n' "$manifest_path"
printf 'para-spec.raw.json sha256: %s\n' "$para_spec_sha256"
printf 'validation-code.wasm sha256: %s\n' "$validation_code_sha256"
printf 'genesis-state.bin sha256: %s\n' "$genesis_state_sha256"
