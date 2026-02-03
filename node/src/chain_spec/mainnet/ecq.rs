use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1005;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "ECQ".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5EeMZNfLW5oywR6FRxrysuTzf7xze1wmxdfqMRCLtxHSayaD
    let root_key =
        AccountId::from(hex!["72227b1b7405e42bb949dc846f424baecf906d3b300b90d5929085fe16fd3d36"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // Collator A
        (
            // 5CcMqPSqAPcwjYmA6GZVRTmVH9WySmxZC9jY5tpugs2Msukn
            AccountId::from(hex![
                "182473dd18521ead358b472bfce693e5e3bafc426ecb46cfcb6e32ed9d8ef44d"
            ]),
            // 5ED24mmTfJ9T2qhBVRuvrvW7PoLG1d4SpoACXxFmUsNAvrG8
            hex!["5ecfb9de50cf880f37e9adbacf43c8dc4c03359aa1fe1a16445dc446af69c704"]
                .unchecked_into(),
        ),
        // Collator B
        (
            // 5Gnf2Xdhz3oLRbCt1Z8rebcX9KyQ2XQKyEojJq6FLSFrYX8X
            AccountId::from(hex![
                "d0efb6d467bb46c1505c9ce9b772209f886bf1816c8fb37a557e104e7579874a"
            ]),
            // 5FpoupmzH9w2S23YvWzxi39pzD2NZDf1ZdZSyZLaHaYAjnkT
            hex!["a657e10d79b10bb81f1539595d7a42e4ddecdf451dfdb684d567cd6bc8fe412d"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "ECQ Security Evidence Chain",
        // ID
        "ecq_mainnet",
        ChainType::Live,
        move || {
            mainnet_genesis(
                Some(root_key.clone()),
                vec![(
                    root_key.clone(),
                    ROOT_STASH - (invulnerables.len() as u128) * COLLATOR_STASH,
                )],
                // initial collators.
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
