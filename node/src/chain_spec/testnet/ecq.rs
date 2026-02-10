use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1007;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "ECQT".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    let root_key =
        AccountId::from(hex!["fef41dd68f783759d1d4be9a9906190dda039bbcd246093db2a8d2e909ee6f4f"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // Albania
        (
            AccountId::from(hex![
                "d0316b8dacb2eb8eec52a6ac7e77bc7ad81678310510eaa170234b2c00208a37"
            ]),
            hex!["dc0773eb9bc37abe3bc25c1a1d893e3efc6877fe70a8b9583441f94c48f57517"]
                .unchecked_into(),
        ),
        // Bahamas
        (
            AccountId::from(hex![
                "34335d162cdb59fd33cd8c0576ca7a592e936bf22243ef7c09579e3f548d8239"
            ]),
            hex!["04abd1620765006e78bf47b89317a885cd6dbb2379682c093a8b71ed47bf5c51"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "ECQ Testnet Chain",
        // ID
        "ecq_testnet",
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
