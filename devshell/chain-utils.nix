{ lib
, writeShellScriptBin
,
}:

{
  build-specs-and-genesis = writeShellScriptBin "build-specs-and-genesis" ''
    declare -A chain_specs

    chain_specs+=(
      ["thx-mainnet"]="mainnet.leafchain.thx"
      ["lmt-mainnet"]="mainnet.leafchain.lmt"

      ["thx-testnet"]="testnet.leafchain.thx"
      ["lmt-testnet"]="testnet.leafchain.lmt"
      ["txd-testnet"]="testnet.leafchain.txd"
      ["sand-testnet"]="testnet.leafchain.sand"
      ["aether-testnet"]="testnet.leafchain.aether"
      ["izutsuya-testnet"]="testnet.leafchain.izutsuya"
    )

    mkdir -pv "dist/chain-specs"
    mkdir -pv "dist/genesis-data"

    for name in ''${!chain_specs[@]}; do
      chain_spec_file_name="dist/chain-specs/''${chain_specs[$name]}.raw.json"
      genesis_state_file="dist/genesis-data/''${chain_specs[$name]}.genesis-state"
      genesis_wasm_file="dist/genesis-data/''${chain_specs[$name]}.genesis-wasm"

      cargo run -- \
        build-spec \
          --disable-default-bootnode \
          --log=error \
          --chain="$name" >"$chain_spec_file_name.origin"

      cargo run -- \
        build-spec \
          --disable-default-bootnode \
          --log=error \
          --raw \
          --chain="$chain_spec_file_name.origin" >"$chain_spec_file_name"

      rm -v "$chain_spec_file_name.origin"

      prettier -w "$chain_spec_file_name"

      cargo run -- \
        export-genesis-state \
          --chain="$name" \
          --log=error > $genesis_state_file

      cargo run -- \
        export-genesis-wasm \
          --chain="$name" \
          --log=error > $genesis_wasm_file

    done
  '';
}

