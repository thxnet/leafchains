{ name
, version
, lib
, rustPlatform
, llvmPackages_15
, protobuf
}:

rustPlatform.buildRustPackage {
  pname = name;
  inherit version;

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "binary-merkle-tree-4.0.0-dev" = "sha256-NdR4/xyoRYe67JHsCit7G95CSTF/TGAJg22NtJU1FP8=";
      "cumulus-client-cli-0.1.0" = "sha256-mlhTYigfROBq11OWZMLwwEyMwE1hp8x+ShMj1mpiH9g=";
      "kusama-runtime-0.9.40" = "sha256-sjamgp7VaL+DeG1gWTFbcz5szjQl2tyfLZH7oTflhcw=";
    };
  };

  nativeBuildInputs = [
    llvmPackages_15.clang
    llvmPackages_15.libclang
  ];

  doCheck = false;

  PROTOC = "${protobuf}/bin/protoc";
  PROTOC_INCLUDE = "${protobuf}/include";

  LIBCLANG_PATH = "${llvmPackages_15.libclang.lib}/lib";

  SUBSTRATE_CLI_GIT_COMMIT_HASH = "";

  CARGO_NET_OFFLINE = "true";
  SKIP_WASM_BUILD = "true";
}
