pub mod aether;
pub mod izutsuya;
pub mod lmt;
pub mod sand;
pub mod thx;
pub mod txd;

use cumulus_primitives_core::ParaId;
use general_runtime::{AccountId, AuraId, Balance, UNITS};

use crate::chain_spec::SAFE_XCM_VERSION;

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
