use cumulus_primitives_core::ParaId;
use sc_chain_spec::Properties;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::crypto::UncheckedInto;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use thxnet_parachain_runtime::{AccountId, AuraId, Balance, Signature, UNITS};
use hex_literal::hex;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
	sc_service::GenericChainSpec<thxnet_parachain_runtime::GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
const COLLATOR_STASH: Balance = 200 * UNITS;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> thxnet_parachain_runtime::SessionKeys {
	thxnet_parachain_runtime::SessionKeys { aura: keys }
}

pub fn thx_testnet_config() -> ChainSpec {
	const PARA_ID: u32 = 1000;
	let mut properties: Properties = Properties::new();
	properties.insert("tokenSymbol".into(), "DEV".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 42.into());

	let extension: Extensions =
		Extensions { relay_chain: "thxnet_testnet".into(), para_id: PARA_ID };

	// 5FpzA56evC5BKCYK2F4uf3Ry6CfUdm3xghBpy5zVdTUqmbKY
	let root_key: AccountId = hex!["a67a5e76bf320f7852fd36f204dffafe2757728be46b12b825f9dead6b95c43e"].into();

	const ROOT_STASH: Balance = 50_000_000_000 * UNITS;

	let invulnerables: Vec<(AccountId, AuraId)> = vec![
		// a
		(
			// 5Enuh6As7rwgMz1h6ua62wq9WT767nArNuByyTaXnucXUdvb
			hex!["78a89d10f59ebf0d9f938b16d9576862f6919e456e93f0d831b347d3f54b402e"].into(),
			// 5G8yCRS86GTBqy8bSAEWy7HCQmBREFiNk4Z7N7xnvM7kcp3P
			hex!["b4318e70ac3a9faea1cdad887f61ca34b3f7fb016199ddf15cb840d113d07831"].unchecked_into(),
		),
		// b
		(
			// 5E2796HJU4oBqwiUSPYBYVmP5vRt6y7VqrXUaS8EdzGJZrds
			hex!["567d1bd9721a4c4a18392ee24452d7df64887ad0b743567915a5c991abbfc94e"].into(),
			// 5HTywmE7ag2aQUVkKSBxJcfciccXWGygEnp8FR5CTvxoYeXB
			hex!["eeedf7d268584a93ddb3536a11f0be8af3803fd84e5719f445656732e5439546"].unchecked_into(),
		),
	];

	ChainSpec::from_genesis(
		// Name
		"thx! token Testnet",
		// ID
		"thx_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				Some(root_key.clone()),
				vec![(root_key.clone(), ROOT_STASH - (invulnerables.len() as u128) * COLLATOR_STASH)],
				// initial collators.
				invulnerables.iter().map(|x| (x.0.clone(), COLLATOR_STASH, x.1.clone())).collect(),
				PARA_ID.into(),
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

fn testnet_genesis(
	root_key: Option<AccountId>,
	endowed_accounts: Vec<(AccountId, Balance)>,
	invulnerables: Vec<(AccountId, Balance, AuraId)>,
	id: ParaId,
) -> thxnet_parachain_runtime::GenesisConfig {
	thxnet_parachain_runtime::GenesisConfig {
		system: thxnet_parachain_runtime::SystemConfig {
			code: thxnet_parachain_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		balances: thxnet_parachain_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.map(|x| (x.0.clone(), x.1))
				.chain(invulnerables.iter().clone().map(|k| (k.0.clone(), k.1)))
				.collect(),
		},
		parachain_info: thxnet_parachain_runtime::ParachainInfoConfig { parachain_id: id },
		collator_selection: thxnet_parachain_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _, _)| acc).collect(),
			candidacy_bond: 100 * UNITS,
			..Default::default()
		},
		session: thxnet_parachain_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, _, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: thxnet_parachain_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
		},
		transaction_payment: Default::default(),
		assets: Default::default(),
		sudo: thxnet_parachain_runtime::SudoConfig { key: root_key },
	}
}
