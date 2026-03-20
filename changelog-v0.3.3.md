# Changelog: v0.3.2 → v0.3.3

> Commits `b528591..e37cb63` | 2026-03-20 | 7 commits

## 1. Add Crowdfunding and RWA Pallets

Added two new runtime pallets to the leafchain:

- **pallet-crowdfunding** — on-chain crowdfunding campaigns with lifecycle management, contribution tracking, and runtime API/RPC endpoints
- **pallet-rwa** — real-world asset tokenization with runtime API/RPC endpoints

Both pallets are integrated into the workspace and `general-runtime`.

### Workspace Members Added

```
pallets/crowdfunding/
pallets/crowdfunding/runtime-api/
pallets/crowdfunding/rpc/
pallets/rwa/
pallets/rwa/runtime-api/
pallets/rwa/rpc/
```

## 2. CI Fixes for Self-hosted Hetzner Runners

Multiple fixes to stabilize CI on the self-hosted Hetzner infrastructure:

| Commit | Fix |
|--------|-----|
| `c053d50` | Switch all workflow runners to self-hosted Hetzner |
| `ce109c4` | Enable Nix experimental features for self-hosted runners |
| `b59f022` | Use `NIX_CONFIG` env var to enable nix-command and flakes |
| `ad27c18` | Use `[hetzner]` runner selector to match rootchain |
| `97d0a70` | Install `xz-utils` before Nix on hetzner runners |
| `e37cb63` | Add Node.js setup before prettier action |

## 3. Version Bump

`Cargo.toml` workspace version: **0.3.2 → 0.3.3**

Affected crates: `thxnet-leafchain`, `general-runtime`, `pallet-crowdfunding`, `pallet-crowdfunding-runtime-api`, `pallet-crowdfunding-rpc`, `pallet-rwa`, `pallet-rwa-runtime-api`, `pallet-rwa-rpc`

---

## Files Changed

```
Cargo.toml                                           # version bump (already 0.3.3)
Cargo.lock                                           # dependency resolution
pallets/crowdfunding/                                # new pallet
pallets/crowdfunding/runtime-api/                    # new runtime API
pallets/crowdfunding/rpc/                            # new RPC
pallets/rwa/                                         # new pallet
pallets/rwa/runtime-api/                             # new runtime API
pallets/rwa/rpc/                                     # new RPC
runtime/general/                                     # pallet integration
.github/workflows/*.yaml                             # CI fixes
```
