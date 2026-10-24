{
  lib,
  stdenvNoCC,
  makeWrapper,
  craneLib,
  cargo-hakari,
  cargo-about,
  microvm-kernel,
  rootfs,
  firecracker,
}:
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
      meta ? { },
      ...
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
            ++ lib.map craneLib.fileset.commonCargoSources (
              [
                ../crates/workspace-hack
                crate
              ]
              ++ extraCrates
            )
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
        meta = {
          mainProgram = pname;
        }
        // meta;
      }
      // lib.removeAttrs args [
        "crate"
        "extraCrates"
        "meta"
      ]
    );

  copyright-html = craneLib.mkCargoDerivation {
    pname = "coden-copyright.html";
    version = "0.0.0";
    src = craneLib.mkDummySrc { inherit src; };
    inherit cargoArtifacts;
    nativeBuildInputs = [ cargo-about ];
    buildPhaseCargoCommand = ''
      mkdir -p "$out"
      cargo about generate -c ${../.config/about.toml} ${../.config/about.hbs} -o "$out/copyright.html"
    '';
  };

  coden-server-unwrapped = buildPackage {
    pname = "coden-server-unwrapped";
    crate = ../crates/coden-server;
    extraCrates = [
      ../crates/coden-vm
      ../crates/coden-agent-api
    ];
    env.COPYRIGHT_HTML = "${copyright-html}/copyright.html";
    meta.mainProgram = "coden-server";
  };
in
{
  inherit coden-server-unwrapped;
  coden-server = stdenvNoCC.mkDerivation {
    pname = "coden-server";
    inherit (coden-server-unwrapped) version;
    nativeBuildInputs = [ makeWrapper ];
    dontUnpack = true;
    buildPhase = ''
      mkdir -p "$out/bin"
      makeWrapper ${lib.getExe coden-server-unwrapped} "$out/bin/coden-server" \
        --set-default VMLINUX ${microvm-kernel} \
        --set-default ROOTFS ${rootfs} \
        --set-default FIRECRACKER ${lib.getExe' firecracker "firecracker"} \
        --set-default FIRECRACKER_JAILER ${lib.getExe' firecracker "jailer"}
    '';
  };

  coden-agent = buildPackage {
    crate = ../crates/coden-agent;
    extraCrates = [ ../crates/coden-agent-api ];
  };

  coden-cargo-checks = {
    clippy = craneLib.cargoClippy (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      }
    );

    rustdoc = craneLib.cargoDoc (
      commonArgs
      // {
        inherit cargoArtifacts;
        env.RUSTDOCFLAGS = "--deny warnings";
      }
    );

    fmt = craneLib.cargoFmt (commonArgs // { inherit src; });

    hakari = craneLib.mkCargoDerivation {
      inherit src;
      pname = "coden-check-hakari";
      version = "0.0.0";
      cargoArtifacts = null;
      doInstallCargoArtifacts = false;

      buildPhaseCargoCommand = ''
        cargo hakari generate --diff
        cargo hakari manage-deps --dry-run
        cargo hakari verify
      '';

      nativeBuildInputs = [
        cargo-hakari
      ];
    };
  };
}
