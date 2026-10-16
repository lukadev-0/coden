{ lib, craneLib }:
let
  src = craneLib.cleanCargoSource ../.;

  commonArgs = {
    pname = "coden";
    version = "0.0.0";
    inherit src;
    strictDeps = true;
    CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
    CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  buildPackage =
    {
      crate,
      extraCrates ? [ ],
    }@args:
    craneLib.buildPackage (
      let
        src = lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions (
            [
              ../Cargo.toml
              ../Cargo.lock
            ]
            ++ lib.map craneLib.fileset.commonCargoSources ([ crate ] ++ extraCrates)
          );
        };
        inherit (craneLib.crateNameFromCargoToml { src = crate; }) pname version;
      in
      commonArgs
      // {
        inherit
          cargoArtifacts
          pname
          version
          src
          ;
        cargoExtraArgs = "-p ${pname}";
      }
      // lib.removeAttrs args [
        "crate"
        "extraCrates"
      ]
    );
in
{
  coden-server = buildPackage {
    crate = ../crates/coden-server;
    extraCrates = [ ../crates/coden-agent-api ];
  };
  coden-agent = buildPackage {
    crate = ../crates/coden-agent;
    extraCrates = [ ../crates/coden-agent-api ];
  };
}
