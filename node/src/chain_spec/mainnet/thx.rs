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
        // a
        (
            // 5F4ufcBdbAjVTND4XSgyx9jqkn2hS97EzZ6WRgjka1kqe6Qc
            AccountId::from(hex![
                "84dc7437003587ba7629e5aa44c16950d1e364dcd9abe4c61fb548a83089ba16"
            ]),
            // 5Fja1rAb6msTeWYUVt3PohWK6zrSZ739AhxiVPnjk4cZjEN5
            hex!["a258df9ed8ba3468f468cc3ac04c372aef8d2f8f1d825eaa9345d3156529ec22"]
                .unchecked_into(),
        ),
        // b
        (
            // 5HZ7eXgS5vorJad9BcxHW1jtzPP8mLhhFvo1dqbKpbLRfHcu
            AccountId::from(hex![
                "f2d8201031504bfd87b9c4f93fddbe0d8855b22100c7bf2c0ec80f12d788260c"
            ]),
            // 5EANEcxgDL5WVqyWhUQSyPjzPgR4GGEgrRmEpkCUqcrX2TpB
            hex!["5cc9e32e038fe57078af7770f81f6be2bf55240ede568b3f4529cd50a94a2957"]
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
