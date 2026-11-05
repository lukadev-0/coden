{ pkgs, craneLib }:
let
  inherit (pkgs) lib;
  src = craneLib.cleanCargoSource ../..;

  commonArgs = {
    pname = "coden";
    version = "0.0.0";
    inherit src;
    strictDeps = true;
    CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
    CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  mkCranePackage =
    {
      crate,
      extraCrates ? [ ],
      meta ? { },
      ...
    }@args:
    let
      src = lib.fileset.toSource {
        root = ../..;
        fileset = lib.fileset.unions (
          [
            ../../Cargo.toml
            ../../Cargo.lock
          ]
          ++ lib.map craneLib.fileset.commonCargoSources (
            [
              ../../crates/workspace-hack
              crate
            ]
            ++ extraCrates
          )
        );
      };
      inherit (craneLib.crateNameFromCargoToml { src = crate; }) pname version;
    in
    craneLib.buildPackage (
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

  coden-copyright-html = craneLib.mkCargoDerivation {
    pname = "coden-copyright.html";
    inherit (commonArgs) version;
    src = craneLib.mkDummySrc { inherit src; };
    inherit cargoArtifacts;
    nativeBuildInputs = [ pkgs.cargo-about ];
    buildPhaseCargoCommand = ''
      mkdir -p "$out"
      cargo about generate -c ${../../.config/about.toml} ${../../.config/about.hbs} -o "$out/copyright.html"
    '';
  };
in
{
  coden-agent = pkgs.callPackage ./coden-agent.nix { inherit mkCranePackage; };
  coden-kernel = pkgs.callPackage ./coden-kernel.nix { };
  coden-rootfs = pkgs.callPackage ./coden-rootfs.nix { };
  coden-rootfs-base = pkgs.callPackage ./coden-rootfs-base.nix { };
  coden-server = pkgs.callPackage ./coden-server.nix { inherit mkCranePackage coden-copyright-html; };
  firecracker = pkgs.callPackage ./firecracker.nix { };

  coden-checks = {
    coden-server-streamLayeredImage = pkgs.coden-server.streamLayeredImage;

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
      pname = "coden-check-hakari";
      inherit (commonArgs) version;
      inherit src;
      cargoArtifacts = null;
      doInstallCargoArtifacts = false;

      buildPhaseCargoCommand = ''
        cargo hakari generate --diff
        cargo hakari manage-deps --dry-run
        cargo hakari verify
      '';

      nativeBuildInputs = [
        pkgs.cargo-hakari
      ];
    };
  };
}
