use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 72_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1001;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "LMT".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5H1U6wwRR2Tak6H4MJu9fW5np3tFjsMfu4rqiFhduRmQwc7e
    let root_key =
        AccountId::from(hex!["dab521b186b518d410bcacbf48951e833d28c652b8c31e924f044715697d695d"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5DhcWUQuTiPREApJqT64SzyGjz7ifTBj2Gd8sDpiMAGKaTe8
            AccountId::from(hex![
                "48624f4e124c96d618ab7efb680c2266c30510242eb0a8578046102f11a7c516"
            ]),
            // 5GjDzYqX6XdiU3kAjWsQ7mByThSwR9S3ufivcEeYRHETWLE1
            hex!["ce51b4a176e15a077f31617e06315557a24cab5d48156dbf4834b464bb774042"]
                .unchecked_into(),
        ),
        // b
        (
            // 5G14MBaLbTVa2R62ms7rsqCB9YwMpz3xtGt8KxH2ATmhjnGZ
            AccountId::from(hex![
                "ae28eee3a8e905bfe636f9c821efa8bc3a9739ca6d98b7e9d1afb39d33648c41"
            ]),
            // 5CcGteMro2LqNDs7zsJeyEXZH6ZRKAonwp4oAmPLPrVt5Hxr
            hex!["1813cf65099decd577ec1fc53aec74c29a236d310ebe1316ddd86cf5adbf552d"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "LimiteT Mainnet",
        // ID
        "lmt_mainnet",
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
