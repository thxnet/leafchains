use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{
    testnet_genesis, ChainSpec, Extensions, COLLATOR_STASH, ROOTCHAIN_TESTNET_NAME,
};

const ROOT_STASH: Balance = 50_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1000;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "DEV".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5Dz8bZ69tXeUXSn4DdPUQy7N7TKhsae5pt5bnkrBR7sQ16Je
    let root_key =
        AccountId::from(hex!["54fb8527957aa0c90898f92c111ea98d007f521feec101677f4f62f3cf5b512a"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5Enuh6As7rwgMz1h6ua62wq9WT767nArNuByyTaXnucXUdvb
            AccountId::from(hex![
                "78a89d10f59ebf0d9f938b16d9576862f6919e456e93f0d831b347d3f54b402e"
            ]),
            // 5G8yCRS86GTBqy8bSAEWy7HCQmBREFiNk4Z7N7xnvM7kcp3P
            hex!["b4318e70ac3a9faea1cdad887f61ca34b3f7fb016199ddf15cb840d113d07831"]
                .unchecked_into(),
        ),
        // b
        (
            // 5E2796HJU4oBqwiUSPYBYVmP5vRt6y7VqrXUaS8EdzGJZrds
            AccountId::from(hex![
                "567d1bd9721a4c4a18392ee24452d7df64887ad0b743567915a5c991abbfc94e"
            ]),
            // 5HTywmE7ag2aQUVkKSBxJcfciccXWGygEnp8FR5CTvxoYeXB
            hex!["eeedf7d268584a93ddb3536a11f0be8af3803fd84e5719f445656732e5439546"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "thx! token Testnet",
        // ID
        "thx_testnet",
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
