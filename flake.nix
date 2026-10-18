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
        import ./nix {
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        } pkgs;

      systems = [ "x86_64-linux" ];
      forEachSystem =
        f:
        lib.genAttrs systems (
          system:
          f system (
            import nixpkgs {
              inherit system;
              overlays = [ (final: _: mkPackages final) ];
            }
          )
        );
    in
    {
      overlays.default = final: _: lib.removeAttrs (mkPackages final) [ "coden-cargo-checks" ];

      packages = forEachSystem (
        system: pkgs: {
          inherit (pkgs)
            firecracker
            rootfs
            microvm-kernel
            coden-server-docker-stream
            coden-server-unwrapped
            coden-server
            coden-agent
            ;
          default = pkgs.coden-server;
        }
      );

      checks = forEachSystem (
        system: pkgs:
        lib.removeAttrs (self.packages.${system}) [ "default" ]
        // lib.mapAttrs' (name: value: {
          name = "coden-cargo-check-${name}";
          inherit value;
        }) (lib.filterAttrs (_: lib.isDerivation) pkgs.coden-cargo-checks)
      );

      devShells = forEachSystem (
        system: pkgs: {
          default = pkgs.mkShell {
            packages = [
              (rustToolchain pkgs)
              pkgs.firecracker
              pkgs.cargo-hakari
            ];
            VMLINUX = pkgs.microvm-kernel;
            ROOTFS = pkgs.rootfs;
            FIRECRACKER = lib.getExe' pkgs.firecracker "firecracker";
            FIRECRACKER_JAILER = lib.getExe' pkgs.firecracker "jailer";
          };
        }
      );
    };
}
