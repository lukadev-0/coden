{
  inputs = {
    nixpkgs = {
      url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    };
    crane = {
      url = "github:ipetkov/crane";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-substituters = [ "https://lukadev.cachix.org" ];
    extra-trusted-public-keys = [ "lukadev.cachix.org-1:RdnilP3lkFbstKV+MkUQAHAUxluyLCwo1wQEMAS9vyQ=" ];
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      self,
    }:
    let
      inherit (nixpkgs) lib;

      rustToolchain =
        pkgs:
        (rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
          targets = [ "x86_64-unknown-linux-musl" ];
        };

      mkPackages =
        pkgs:
        import ./nix/packages {
          inherit pkgs;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        };

      packageNames = [
        "coden-agent"
        "coden-kernel"
        "coden-rootfs"
        "coden-rootfs-base"
        "coden-server"
        "firecracker"
      ];

      systems = lib.genAttrs [ "x86_64-linux" ] (
        system:
        import nixpkgs {
          inherit system;
          overlays = [ (final: _: mkPackages final) ];
        }
      );
      forEachSystem = f: lib.mapAttrs f systems;
    in
    {
      overlays.default = final: _: lib.getAttrs packageNames (mkPackages final);

      packages = forEachSystem (
        system: pkgs:
        lib.getAttrs packageNames pkgs
        // {
          default = pkgs.coden-server;
          busybox-static = pkgs.pkgsStatic.busybox;
        }
      );

      checks = forEachSystem (
        system: pkgs:
        lib.getAttrs packageNames pkgs
        // lib.mapAttrs' (name: value: {
          name = "coden-check-${name}";
          inherit value;
        }) pkgs.coden-checks
      );

      devShells = forEachSystem (
        system: pkgs: {
          default = pkgs.mkShell {
            packages = [
              (rustToolchain pkgs)
              pkgs.firecracker
              pkgs.cargo-hakari
              pkgs.cargo-about
              pkgs.e2fsprogs
            ];
            VMLINUX = "${pkgs.coden-kernel}/kernel";
            FIRECRACKER = lib.getExe' pkgs.firecracker "firecracker";
            FIRECRACKER_JAILER = lib.getExe' pkgs.firecracker "jailer";
          };
        }
      );
    };
}
