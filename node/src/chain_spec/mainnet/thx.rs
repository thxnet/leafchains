use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 50_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1000;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "thx!".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5G1JJF5dLcKFWmp6JQpX2hEXa94SJpakGwg3waaMTVito9UB
    let root_key =
        AccountId::from(hex!["ae57e4083e13199fe977de0ccbdbecee0f5cfc841fceb1685f10ff8a46e0f811"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        (
            // a
            // 5GxXcFqMWE2BrpjYVuH4V34sUjEbt6CnF5QZiERe3YpKJpVP
            AccountId::from(hex![
                "d87732031b096246ca653c99875b28b3e8c31ba2b746efaddd7634cfd2f31933"
            ]),
            // a//aura
            // 5DcRubWqTRQGryiuEaBAGLmRFvFZUp3hmoRJ1R7KCh5XbBou
            hex!["446e653e4847155a782cccf12c28e2bdb0f465e46e408bb86e306aa3ee54126e"]
                .unchecked_into(),
        ),
        (
            // b
            // 5HKxBfwzYJZN5E212aPHHPpXoUVCgGenWk5TAMpaEpHuk16x
            AccountId::from(hex![
                "e8ce1459a5845060df05b8281d91b78c669f281e259da169d2c5702670659f44"
            ]),
            // b//aura
            // 5CQ6QpRHX9Pa93E87REwdoaevPe7zqVwBGJ2TTLHEimm8rgF
            hex!["0ec9965539c01f0742e074f1fa117095a3984bacbe26073277b320d22912be02"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "thx! token Mainnet",
        // ID
        "thx_mainnet",
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
