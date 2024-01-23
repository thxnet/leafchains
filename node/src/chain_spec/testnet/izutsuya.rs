use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1005;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "#Z28".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5HRLU6qM1nQ7xs6kQeJzUJMsnQniSQDyvojKMZp34E4FsYVg
    let root_key =
        AccountId::from(hex!["ece9519342d62a97c290331f4bdcc53f64a2b8e59ec99725ae4d189334f4c842"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5F9XvnZDcnR63xQukn8yRYPQan5h2us3miGkCrKqKCrtwXSD
            AccountId::from(hex![
                "886380dc1aaad24f1b47fabb2f1b4e4deb04c2dba65d68e18981ce3a6fda3505"
            ]),
            // 5DvemxmNsxmBPYFv1xDSJoNhhjqya3c7NyFkAsMyj5frEz2k
            hex!["525421b12220da6d178ecebf22a695c68a3059637737531f316d42c6aa9ba449"]
                .unchecked_into(),
        ),
        // b
        (
            // 5G1YXzp77Hw3GzcnxPw3iDEqCgQER8rv5hpW3dcwTygzPMxj
            AccountId::from(hex![
                "ae87d135f6629b4813dbeaefbc2e2cbe4b440958cfcf0ef7ab0f65ef05570008"
            ]),
            // 5Ca3apcLH57Pf9irr25j6wvhGdrGqVXa71YpBz2SgsokTqSr
            hex!["166085e1437ed07f2228d433520a1c572972cfc81642842f8b7f30c08ddcc66d"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "IZUTSUYA Testnet",
        // ID
        "izutsuya_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
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
