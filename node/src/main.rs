//! THXNET. Leafchain Node

#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod fork_genesis_cmd;
mod rpc;

fn main() -> sc_cli::Result<()> { command::run() }
