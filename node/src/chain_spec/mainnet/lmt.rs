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
        (
            // a
            // 5CSD4qABAjqujPzEFrWrAZqXX3s6s3KoG6zWSU8AaKrYd5DH
            AccountId::from(hex![
                "10667b6a841e2caeec8a624b2e2943b1e7c6af3a8769da406334576591d94856"
            ]),
            // a//aura
            // 5F4gReRDEV2c6Lx98gcKE3DCD5RP9V95vHyDvXCe6XPbbPX7
            hex!["84afe1b9f54099ac376fd55d96ceecdc81bea0cb07b3987bc00931293c51dc4f"]
                .unchecked_into(),
        ),
        (
            // b
            // 5Fbh3BZKUpVVvy8RzdprS3WvN8fNbL1W2XEgEBmJc5mJxC3n
            AccountId::from(hex![
                "9c568d4499c568214ef947ac8d7e222f7447490c2e8163fec5f01159a18a7f07"
            ]),
            // b//aura
            // 5CCkGiwkZKR1nUaZaG6hRobg4DfwLzxb8258Ypp6jPAYg1Tb
            hex!["0622178cd611eb934681d2bcbf81db1c40e4ff5b4ec6160a44d214b9deb2dd71"]
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
