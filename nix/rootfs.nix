{
  lib,
  runCommand,
  writeClosure,
  writeTextDir,
  buildEnv,
  squashfsTools,
  busybox,
  coden-agent,
}:
let
  busybox' = lib.getExe' busybox;

  etc = {
    passwd = ''
      root:x:0:0::/root:${busybox' "nologin"}
      runner:x:1000:1000::/runner:${busybox' "sh"}
    '';
    group = ''
      root:x:0:root
      runner:x:1000:runner
    '';
    inittab = ''
      ::ctrlaltdel:${busybox' "reboot"}
      ::once:${lib.getExe coden-agent}
      ::restart:${busybox' "init"}
    '';
  };

  envPackages = [
    busybox
  ]
  ++ lib.mapAttrsToList (name: contents: writeTextDir "etc/${name}" contents) etc;

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
  ''
