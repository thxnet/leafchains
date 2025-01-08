use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{mainnet::mainnet_genesis, ChainSpec, Extensions, ROOTCHAIN_MAINNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1002;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn mainnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "ACTI".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_MAINNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5CM5tbsVTDWwU2pmWpTdyMkjnVucE3oPw9JMS8TWAKsnSy9K
    let root_key =
        AccountId::from(hex!["0c7e18e73cc90ee30078f67470b25dae7ef436b140a7f158edbd9019e69bef29"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        (
            // a
            // 5FhKohJXX98oKAi7n2kNEYvswsWQVoE7WdxyMhgCVHJAE5vV
            AccountId::from(hex![
                "a0a28c95ed65e109636d8534a89f88432f66fed43e3f22ce150c13d128cdd623"
            ]),
            // a//aura
            // 5GWRua8Et45j9WtCHgwyH52MFxAuag486NUeVBfgwXZ777qT
            hex!["c48f9f3cf432f474d2be49c963746f4fca8b5b86ad6c6306d3dd42f8ed3f482c"]
                .unchecked_into(),
        ),
        (
            // b
            // 5DS87vUCabhF5gEQF4Pmcb34aJFJ2oHWxGSuNxS4qNYiwdFx
            AccountId::from(hex![
                "3c9210403a16e5004edb7cc4ce4b2100699e1bfb3383b50d581550bd35da331d"
            ]),
            // b//aura
            // 5Gj8Pjfv17CMrrtZXVYq8mLZqXb6FH4UnAC5qLrUgFAn4gNH
            hex!["ce3eda81bde2626b135e6d00656bff8394f3179012119c592a3d422a8b36ad29"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "Activa",
        // ID
        "activa_mainnet",
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
