{ craneLib }:
pkgs: {
  firecracker = pkgs.callPackage ./firecracker.nix { };
  rootfs-base = pkgs.callPackage ./rootfs-base.nix { };
  rootfs = pkgs.callPackage ./rootfs.nix { };
  microvm-kernel = pkgs.callPackage ./microvm-kernel.nix { };
  coden-server-docker-stream = pkgs.callPackage ./docker-image.nix { };
  inherit
    (pkgs.callPackages ./crates.nix {
      inherit craneLib;
    })
    coden-server-unwrapped
    coden-server
    coden-agent
    coden-cargo-checks
    ;
}
