pub mod lmt;
pub mod thx;

use cumulus_primitives_core::ParaId;
use general_runtime::{AccountId, AuraId, Balance, UNITS};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<general_runtime::GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
const COLLATOR_STASH: Balance = 200 * UNITS;
const RELAY_CHAIN_NAME: &str = "thxnet_testnet";

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

fn testnet_genesis(
    root_key: Option<AccountId>,
    endowed_accounts: Vec<(AccountId, Balance)>,
    invulnerables: Vec<(AccountId, Balance, AuraId)>,
    id: ParaId,
) -> general_runtime::GenesisConfig {
    general_runtime::GenesisConfig {
        system: general_runtime::SystemConfig {
            code: general_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        balances: general_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .map(|x| (x.0.clone(), x.1))
                .chain(invulnerables.iter().clone().map(|k| (k.0.clone(), k.1)))
                .collect(),
        },
        parachain_info: general_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: general_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, ..)| acc).collect(),
            candidacy_bond: 100 * UNITS,
            ..Default::default()
        },
        session: general_runtime::SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, _, aura)| {
                    (
                        acc.clone(),                           // account id
                        acc,                                   // validator id
                        general_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        polkadot_xcm: general_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
        },
        transaction_payment: Default::default(),
        assets: Default::default(),
        sudo: general_runtime::SudoConfig { key: root_key },
    }
}
