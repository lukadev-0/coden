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

      systems = [ "x86_64-linux" ];
      forEachSystem =
        f:
        lib.genAttrs systems (
          system:
          f system (
            import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            }
          )
        );
    in
    {
      overlays.default =
        final: _:
        import ./nix {
          craneLib = (crane.mkLib final).overrideToolchain rustToolchain;
        } final;

      packages = forEachSystem (
        system: pkgs: {
          inherit (pkgs)
            firecracker
            rootfs
            microvm-kernel
            coden-server-docker-stream
            coden-server
            coden-agent
            ;
          default = pkgs.coden-server;
        }
      );

      devShells = forEachSystem (
        system: pkgs: {
          default = pkgs.mkShell {
            packages = [
              (rustToolchain pkgs)
              pkgs.firecracker
            ];
            VMLINUX = "${pkgs.microvm-kernel.dev}/vmlinux";
            ROOTFS = pkgs.rootfs;
            FIRECRACKER = lib.getExe' pkgs.firecracker "firecracker";
            FIRECRACKER_JAILER = lib.getExe' pkgs.firecracker "jailer";
          };
        }
      );
    };
}
