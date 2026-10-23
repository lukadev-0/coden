{
  lib,
  writeTextDir,
  rootfs-base,
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
      ::sysinit:${busybox' "mount"} -t proc proc /proc
      ::sysinit:${busybox' "mount"} -t sysfs sysfs /sys
      ::sysinit:${busybox' "mount"} -t tmpfs -o size=5M tmpfs /runner
      ::sysinit:${busybox' "mount"} -t tmpfs -o size=5M tmpfs /tmp
      ::once:${lib.getExe coden-agent}
      ::ctrlaltdel:${busybox' "reboot"}
      ::restart:${busybox' "init"}
    '';
  };

  additionalPackages = lib.mapAttrsToList (name: contents: writeTextDir "etc/${name}" contents) etc;
in
rootfs-base.override {
  inherit additionalPackages;
}
