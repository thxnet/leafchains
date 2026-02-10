---
name: add-leafchain
description: Add a new leafchain (testnet or mainnet), generating the chain_spec and modifying all related files
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, AskUserQuestion
argument-hint: "[testnet|mainnet] [chain-name] (or provide a spec file path)"
---

# Add Leafchain

This skill adds a new leafchain to the THXNET Leafchains project.

## Workflow

### Step 0: Gather parameters

Confirm the following required information with the user (if a spec file is provided, read from it):

| Parameter      | Description                                                                                         | Example                     |
| -------------- | --------------------------------------------------------------------------------------------------- | --------------------------- |
| Network Type   | `testnet` or `mainnet`                                                                              | testnet                     |
| Chain Name     | Display name for the chain                                                                          | ECQ Security Evidence Chain |
| Chain ID slug  | Identifier used with `--chain=` (format: `{name}-testnet` or `{name}-mainnet`)                      | ecq-testnet                 |
| Module Name    | Rust module name (snake_case, used for filename and mod.rs)                                         | ecq                         |
| Token Symbol   | Token symbol                                                                                        | ECQT                        |
| Token Decimals | Decimal places                                                                                      | 10                          |
| Initial Supply | Initial supply in UNITS (e.g. `1_000_000_000`)                                                      | 1_000_000_000               |
| Leafchain ID   | Parachain ID (u32)                                                                                  | 1007                        |
| Sudo Key       | Admin account public key (hex, without 0x prefix), with the corresponding SS58 address as a comment | 265abf0b6e...               |
| Collators      | Each collator's AccountId (hex) + AuraId (hex), with their respective SS58 addresses as comments    | ...                         |

Use the `AskUserQuestion` tool to prompt for any missing parameters.

### Step 1: Create chain_spec file

Choose the template based on network type:

- **testnet**: Use `node/src/chain_spec/testnet/aether.rs` as the template
- **mainnet**: Use `node/src/chain_spec/mainnet/ecq.rs` as the template

Create a new file at `node/src/chain_spec/{network}/{module_name}.rs`.

#### Testnet template structure

```rust
use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = {initial_supply} * UNITS;
const LEAFCHAIN_ID: u32 = {leafchain_id};
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "{token_symbol}".into());
    properties.insert("tokenDecimals".into(), {token_decimals}.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // {sudo_ss58_address}
    let root_key =
        AccountId::from(hex!["{sudo_key_hex}"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // ... collators
    ];

    ChainSpec::from_genesis(
        // Name
        "{chain_name}",
        // ID
        "{chain_id_underscore}",  // e.g. "ecq_testnet" (replace - with _ in slug)
        ChainType::Live,
        move || {
            testnet_genesis(
                Some(root_key.clone()),
                vec![(
                    root_key.clone(),
                    ROOT_STASH - (invulnerables.len() as u128) * COLLATOR_STASH,
                )],
                invulnerables.iter().map(|x| (x.0.clone(), COLLATOR_STASH, x.1.clone())).collect(),
                LEAFCHAIN_ID.into(),
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(properties),
        extension,
    )
}
```

#### Mainnet template structure

Same as testnet, with the following differences:

- Import uses `mainnet::mainnet_genesis` instead of `testnet::testnet_genesis`
- Import uses `ROOTCHAIN_MAINNET_NAME` instead of `ROOTCHAIN_TESTNET_NAME`
- Function is named `mainnet_config()` instead of `testnet_config()`
- Internally calls `mainnet_genesis(...)` instead of `testnet_genesis(...)`

#### Collator format

Each collator follows this format (note the SS58 addresses as comments):

```rust
// {collator_label}
(
    // {account_ss58_address}
    AccountId::from(hex![
        "{account_id_hex}"
    ]),
    // {aura_ss58_address}
    hex!["{aura_id_hex}"]
        .unchecked_into(),
),
```

### Step 2: Update mod.rs

Add `pub mod {module_name};` to `node/src/chain_spec/{network}/mod.rs`.

**Important**: Insert in alphabetical order, maintaining consistency with the existing module declarations.

### Step 3: Update command.rs

Add a new match arm to the `load_spec` function in `node/src/command.rs`, in the appropriate network section.

- **testnet section**: Under the `// testnet` comment, add in alphabetical order:

  ```rust
  "{chain_id_slug}" => Box::new(chain_spec::testnet::{module_name}::testnet_config()),
  ```

- **mainnet section**: Under the `// mainnet` comment, add in alphabetical order:
  ```rust
  "{chain_id_slug}" => Box::new(chain_spec::mainnet::{module_name}::mainnet_config()),
  ```

Note: If the chain ID slug contains `-` and the module name uses `_` (e.g. `mirrored-body-testnet` maps to `mirrored_body`), the match arm string uses the slug (with `-`), while the module path uses the module name (with `_`).

### Step 4: Update Containerfile

Add a new entry to the `chain_specs` associative array in `dev-support/containers/debian/builder/Containerfile`.

Format:

```bash
["{chain_id_slug}"]="{network}.leafchain.{module_name}"
```

Add the new entry to the corresponding network block (mainnet entries together, testnet entries together), keeping the layout tidy.

### Step 5: Update chain-utils.nix

Add a new entry to the `chain_specs` associative array in `devshell/chain-utils.nix`, using the same format as the Containerfile:

```bash
["{chain_id_slug}"]="{network}.leafchain.{module_name}"
```

Similarly, add the new entry to the corresponding network block.

### Step 6: Verify

Run the following command to confirm the build passes:

```bash
cargo check
```

If the build fails, fix the issues based on the error messages and re-check.

## Notes

- All hex values should be without the `0x` prefix
- Chain ID uses `_` as separator in `from_genesis` (e.g. `ecq_testnet`), but uses `-` in the `load_spec` match arm (e.g. `ecq-testnet`)
- `ss58Format` is always 42
- `COLLATOR_STASH` is always `200 * UNITS`
- `ChainType` is always `ChainType::Live`
