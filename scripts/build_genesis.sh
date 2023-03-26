#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
cd $PROJECT_ROOT

./target/release/parachain-template-node export-genesis-state > genesis-state
./target/release/parachain-template-node export-genesis-wasm > genesis-wasm

popd