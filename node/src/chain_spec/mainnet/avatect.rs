use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1004;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "AVATC".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5F1ZF35R5dzb3vwU9r4FQ25pfvLn47a17k89N6fxmET3xCDh
    let root_key =
        AccountId::from(hex!["824df736064474971a36a5747c5ddb233e0dfad78af19223a6ff8a8c45d54a1f"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5GTcgzts2nUyDVDSw6eYpFy25C3K9YW9K23N5YwWSpkAgLY9
            AccountId::from(hex![
                "c26a2f75379dce4cea0218b3c86086b9328487cf0dfc915bfc2de1398e8b0777"
            ]),
            // 5EWdzy6cBj5jeqdNWjKqAC83w9zQiVhRP1yoe1dFmRwF6yeQ
            hex!["6c3fdd28da565b835b20568d0b08076719a6bea5b53c6594503e142c08402003"]
                .unchecked_into(),
        ),
        // b
        (
            // 5GvAiBM6DbThERSQF891udzqKeKBHrmUN4BSHVoZYFsTWDoK
            AccountId::from(hex![
                "d6aa5b22a121e9b5901189757f7090f71c5a4f94bd1de4056dad666d43fed321"
            ]),
            // 5DvWFLG9H9FLTLX9DKwiR6brAP6rNNHEmtysavGjqRjQrBfd
            hex!["52376c69d3b8137b39cbd36587cbe968fa46b29d1c56b28852f2516f9390d500"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "AVATECT",
        // ID
        "avatect_mainnet",
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
