use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1003;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "MBT".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5DstaKNkHeVrA1riHsuJzt1ZkoZpNm7rTWtqDR6Gg6zeDrJ3
    let root_key =
        AccountId::from(hex!["5038d905d262111acf63d524b5f8d9d98d6bbf518d61c7b2635a965ca62a7433"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5EsfEnoCQYoeLeMzUiHAbfEBRwUi6SzKZnaScpwgswRBA1M9
            AccountId::from(hex![
                "7c482f9a0b0edb03f6029c8efab71df99710e45c97dd6e62aa1c6e952e51c571"
            ]),
            // 5DaXnJV3RUULGoHqwc4xnhdeo1HLbEQmPvRPMDrsTj5XNz4a
            hex!["42fbaddcfb5a2dbafcd314df98af2bd2fedb04d377b454d1963eb6ac6b581d3e"]
                .unchecked_into(),
        ),
        // b
        (
            // 5Gs63XE3Dd8GYRzpWeuL9H5FeDerCV3jDep4XuFMmAksnfWi
            AccountId::from(hex![
                "d450e932a787c2c6ebbdef3a88a35b20eb56342bc6adf2a933358199d2075354"
            ]),
            // 5DPGCPztkLd4BzUHV2tufrd8ikaopXUiq8icWVkDozgU1Gg1
            hex!["3a638472a23b6b83b241cb1b6e4213eac5590a6a01786c4f5a5f215069439953"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "Mirrored Body",
        // ID
        "mirrored_body_mainnet",
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
