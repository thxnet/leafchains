use general_runtime::{AccountId, AuraId, Balance, UNITS};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::crypto::UncheckedInto;

use crate::chain_spec::{testnet::testnet_genesis, ChainSpec, Extensions, ROOTCHAIN_TESTNET_NAME};

const ROOT_STASH: Balance = 1_000_000_000 * UNITS;
const LEAFCHAIN_ID: u32 = 1006;
const COLLATOR_STASH: Balance = 200 * UNITS;

pub fn testnet_config() -> ChainSpec {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "MBTT".into());
    properties.insert("tokenDecimals".into(), 10.into());
    properties.insert("ss58Format".into(), 42.into());

    let extension =
        Extensions { rootchain: ROOTCHAIN_TESTNET_NAME.to_string(), leafchain_id: LEAFCHAIN_ID };

    // 5G4ARte1S8GZ61RjRHU9cenMuoYUy8BWvNRpCGP2U7Cn75Fd
    let root_key =
        AccountId::from(hex!["b08723f2e18e429ab5835fe9d4cfedfc0771d27c1d56bb2846530f874622d278"]);

    let invulnerables: Vec<(AccountId, AuraId)> = vec![
        // a
        (
            // 5Dt6thrDKzt6gPRVMYS1et67kYEFGSGVUmMW1YA6uJAfYqrp
            AccountId::from(hex![
                "50624f801f053fbc58f03a5a8ff07d409abf5e64e78979593c9d44dc31f85c40"
            ]),
            // 5EwAemSCadDSfKEHS53PyELRTekRGn4QFiPEC7G2Gm3Yfzu2
            hex!["7ef4efbb13fe8a60e84d3cb2ffa28f77e9853b1321124a4257b8cae7b2e26652"]
                .unchecked_into(),
        ),
        // b
        (
            // 5DSgB9vq5zGmVfdUtzYHGzwaAA3bZZSRttehsNZMSdGXvmEb
            AccountId::from(hex![
                "3cfdf900e22d828d0e16565d6cf7904e39581821a6714bb5e33c1ea89ef3ba4f"
            ]),
            // 5Fk7pXQqChJNA6c2JJvz3S4LjfkgfrycjgTpD3SRaxYd4qGV
            hex!["a2c3f01b4b8c3494fe9c2e312ff7db1cc37fde6f271ec29dabe430b09584a95d"]
                .unchecked_into(),
        ),
    ];

    ChainSpec::from_genesis(
        // Name
        "Mirrored Body Test",
        // ID
        "mirrored_body_testnet",
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
