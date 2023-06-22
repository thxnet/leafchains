{ rustToolchain
, cargoArgs
, unitTestArgs
, lib
, stdenv
, pkgs
, ...
}:

let
  cargo-ext = pkgs.callPackage ./cargo-ext.nix { inherit cargoArgs unitTestArgs; };
  chain-utils = pkgs.callPackage ./chain-utils.nix { };
in
pkgs.mkShell {
  name = "dev-shell";

  nativeBuildInputs = with pkgs; [
    cargo-ext.cargo-build-all
    cargo-ext.cargo-clippy-all
    cargo-ext.cargo-doc-all
    cargo-ext.cargo-nextest-all
    cargo-ext.cargo-test-all
    cargo-ext.cargo-udeps-all
    cargo-ext.cargo-watch-all
    cargo-nextest
    cargo-udeps
    cargo-watch
    rustToolchain
    chain-utils.build-specs-and-genesis

    tokei

    llvmPackages_15.clang
    llvmPackages_15.libclang

    protobuf

    treefmt

    jq
    nixpkgs-fmt
    shfmt
    nodePackages.prettier
    shellcheck
  ] ++ lib.optionals stdenv.isDarwin [
    iconv

    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  PROTOC = "${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE = "${pkgs.protobuf}/include";

  LIBCLANG_PATH = "${pkgs.llvmPackages_15.libclang.lib}/lib";

  shellHook = ''
    export NIX_PATH="nixpkgs=${pkgs.path}"
  '';
}
