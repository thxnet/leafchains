{ name
, version
, lib
, rustPlatform
, llvmPackages
, mold
, protobuf
}:

rustPlatform.buildRustPackage {
  pname = name;
  inherit version;

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "binary-merkle-tree-4.0.0-dev" = "sha256-YxCAFrLWTmGjTFzNkyjE+DNs2cl4IjAlB7qz0KPN1vE=";
      "cumulus-client-cli-0.1.0" = "sha256-JjSquK6NjYt8X2PNxlNw0FuOh+RwlB0k2Su6AFSWr7I=";
      "kusama-runtime-0.9.40" = "sha256-xpor2sWdYD9WTtmPuxvC9MRRLPPMk8yHlD7RwtSijqQ=";
    };
  };

  nativeBuildInputs = [
    llvmPackages.clang
    llvmPackages.libclang

    mold
    protobuf
  ];

  doCheck = false;

  PROTOC = "${protobuf}/bin/protoc";
  PROTOC_INCLUDE = "${protobuf}/include";

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

  SUBSTRATE_CLI_GIT_COMMIT_HASH = "";

  # NOTE: We don't build the WASM runtimes since this would require a more
  # complicated rust environment setup and this is only needed for developer
  # environments. The resulting binary is useful for end-users of live networks
  # since those just use the WASM blob from the network chainspec.
  # See also: https://docs.rs/substrate-wasm-builder/latest/substrate_wasm_builder/#environment-variables
  SKIP_WASM_BUILD = 1;
}
