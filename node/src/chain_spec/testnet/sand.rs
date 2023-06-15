use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = 10_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1003;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "SAND".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5Fk5MhHQrWXWs5yGz4bAF48ipEs8YZ6Pvax82SUhKC2CnrNj
    let root_key =
        AccountId::from(hex!["a2bba5eb7baf43cf38be7a891997896b26ae13996a8afe4de6013d741c11fa70"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5DfVDz6FDPivaKxE8rdpUnRA3i47XWud1Dn7ZYDeJdUF4TRu
            AccountId::from(hex![
                "46c34c432fe74bdd24a6c0de7d7da94cb3128690198f35415e777547dd00c532"
            ]),
            // 5Eh99wpS6pDo7CsbRKpULZdiBtEdYWjZpwQsKrQvSgRmqnCK
            hex!["74427633c777a7bb4ee971b07fc96c0b66218a328ddb27380356ef2439e8f94f"]
                .unchecked_into(),
        ),
        // b
        (
            // 5ETk5ZKvozzH38v4SBAia3gcwQW2828doEJ7xJrnXBfnWYkN
            AccountId::from(hex![
                "6a0a9786fffda330b5bb9d8c81753382dc8b2c090fbc1b398dee58053b549c53"
            ]),
            // 5EcEY3LrcDCyhtcNKPri4rEVq1q8gL6PWenN6AXpFL7kvMuz
            hex!["2ff069cc3362a42c6593b57f810a10cf5464fb0b76b12e19bc46eea59465bf72"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "Sandbox",
        // ID
        "sand_testnet",
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
