{
  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
  };

  outputs =
    { nixpkgs, self }:
    let
      inherit (nixpkgs) lib;

      firecracker =
        { stdenvNoCC, fetchurl }:
        let
          hashes = {
            x86_64-linux = "sha256-BglKEQiunoKqTCOndaqSdY9T8RddQiJw2dYWLLmt5Vg=";
          };
          arch = stdenvNoCC.hostPlatform.parsed.cpu.name;
        in
        stdenvNoCC.mkDerivation (
          finalAttrs:
          let
            inherit (finalAttrs) version;
          in
          {
            pname = "firecracker";
            version = "1.17.0";
            src = fetchurl {
              url = "https://github.com/firecracker-microvm/firecracker/releases/download/v${version}/firecracker-v${version}-${arch}.tgz";
              hash = hashes.${stdenvNoCC.hostPlatform.system};
            };
            installPhase = ''
              mkdir -p "$out/bin"
              cp firecracker-v${version}-${arch} "$out/bin/firecracker"
              cp jailer-v${version}-${arch} "$out/bin/jailer"
            '';
            meta.mainProgram = "firecracker";
          }
        );

      rootfs =
        {
          runCommand,
          writeClosure,
          writeTextDir,
          buildEnv,
          squashfsTools,
          busybox,
        }:
        let
          busybox' = lib.getExe' busybox;

          envPackages = [
            busybox
            (writeTextDir "etc/passwd" ''
              root:x:0:0::/root:${busybox' "nologin"}
              runner:x:1000:1000::/runner:${busybox' "sh"}
            '')
            (writeTextDir "etc/group" ''
              root:x:0:root
              runner:x:1000:runner
            '')
          ];

          rootfsEnv = buildEnv {
            name = "rootfs-env";
            paths = envPackages;
            pathsToLink = [
              "/bin"
              "/etc"
            ];
          };
        in
        runCommand "rootfs"
          {
            inherit rootfsEnv;
            __structuredAttrs = true;
            unsafeDiscardReferences.out = true;
            closure = writeClosure envPackages;
            nativeBuildInputs = [ squashfsTools ];
          }
          ''
            sourceFiles=()

            addFile() {
              sourceFiles+=("$1")
              echo "add $path"
            }

            while read -r path; do
              addFile "$path"
            done < "$closure"

            cd "$rootfsEnv"
            for path in *; do
              addFile "$path"
            done

            mksquashfs "''${sourceFiles[@]}" "$out" \
              -no-strip -force-uid 0 -force-gid 0 -repro -info -pseudo-override \
              -root-mode 755 \
              -p "/root d 700 0 0" \
              -p "/runner d 700 1000 1000" \
              -p "/dev d 755 0 0" \
              -p "/proc d 555 0 0" \
              -p "/sys d 555 0 0" \
              -p "/tmp d 1777 0 0"
          '';

      microvm-kernel =
        { linux_6_18, linuxManualConfig }:
        let
          linux = linux_6_18;
        in
        linuxManualConfig {
          inherit (linux) src version;
          configfile = ./microvm-kernel.config;
        };

      coden =
        { rustPlatform }:
        rustPlatform.buildRustPackage {
          pname = "coden";
          version = "0.0.0";
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.lock
              ./Cargo.toml
              ./src
            ];
          };
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };

      overlay = final: _: {
        firecracker = final.callPackage firecracker { };
        rootfs = final.callPackage rootfs { };
        microvm-kernel = final.callPackage microvm-kernel { };
        coden = final.callPackage coden { };
      };

      systems = [
        "x86_64-linux"
      ];
      forEachSystem =
        f:
        lib.genAttrs systems (
          system:
          f {
            inherit system;
            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                overlay
              ];
            };
          }
        );
    in
    {
      overlays.default = overlay;

      packages = forEachSystem (
        { pkgs, ... }: {
          inherit (pkgs)
            microvm-kernel
            firecracker
            rootfs
            coden
            ;
          coden-docker = pkgs.dockerTools.streamLayeredImage {
            name = "coden";
            contents = pkgs.pkgsStatic.coden;
            extraCommands = ''
              cp ${pkgs.rootfs} rootfs
              cp ${pkgs.microvm-kernel.dev}/vmlinux vmlinux
              mkdir tmp
            '';
            config = {
              Env = [
                "VMLINUX=/vmlinux"
                "ROOTFS=/rootfs"
                "FIRECRACKER=${lib.getExe' pkgs.firecracker "firecracker"}"
                "FIRECRACKER_JAILER=${lib.getExe' pkgs.firecracker "jailer"}"
              ];
              Entrypoint = [ "/bin/coden" ];
            };
          };
          default = pkgs.coden;
        }
      );

      devShells = forEachSystem (
        { pkgs, ... }:
        {
          default = pkgs.mkShell {
            packages = [ pkgs.firecracker ];
            VMLINUX = "${pkgs.microvm-kernel.dev}/vmlinux";
            ROOTFS = pkgs.rootfs;
            FIRECRACKER = lib.getExe' pkgs.firecracker "firecracker";
            FIRECRACKER_JAILER = lib.getExe' pkgs.firecracker "jailer";
          };
        }
      );
    };
}
