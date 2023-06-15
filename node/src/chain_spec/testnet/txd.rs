use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = 10_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1002;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "DEVTXD".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    //  5DZsrsCEP2A6z2Up8oevjm71j8JbMbbiScjUZPeS5sTPeYBa
    let root_key =
        AccountId::from(hex!["427c064ee6f855bb127cfcead620ff701887a8381b05cfa45f59ac13c3283845"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            //  5GWX5fXJ4oQ22Mznnn8gV6ns3T3kBXeQNheVHU1J9LiYnUXz
            AccountId::from(hex![
                "c4a10a11010ee2aa9d6c6e18505ae4e17a62f73e6432cba766d510f4ea4f3a64"
            ]),
            // 5E4Yd6pEoQvP4D7GhBTE5BVYuKv6P2YYXTqYTU7LjmTRdBDx
            hex!["585961fb57fc7d67334f53e8ecde94a7162160b3a7721cdac85dbaf9fa59792e"]
                .unchecked_into(),
        ),
        // b
        (
            // 5HeC2i9Do5pqByt1CXfQLJhQFPVH1W9hovX8h3Hi4qvzitkg
            AccountId::from(hex![
                "f6b71ad4dee38beef3ac94674ede3c37c71e933eae8eeb66ffa3d51300e1b42b"
            ]),
            // 5CDRGLYnPsCP6jPP5efitiR6kGF8tvFhSWdGncTN9CaXaXGQ
            hex!["06a55af3ea5af23c8c65e7e1b86c4b187fdbf6a56c172dfc9e4a33f159f41670"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "TXD Testnet",
        // ID
        "txd_testnet",
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
