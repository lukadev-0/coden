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

  crateArgs =
    crate:
    let
      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          ../Cargo.toml
          ../Cargo.lock
          (craneLib.fileset.commonCargoSources crate)
        ];
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
    };
in
{
  coden-server = craneLib.buildPackage (crateArgs ../crates/coden-server);
  coden-agent = craneLib.buildPackage (crateArgs ../crates/coden-agent);
}
