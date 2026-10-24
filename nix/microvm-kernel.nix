{
  linux_6_18,
  linuxManualConfig,
  runCommand,
}:
let
  base = linux_6_18;
  linux = linuxManualConfig {
    inherit (base) src;
    version = "${base.version}-coden";
    configfile = ./microvm-kernel.config;
  };
in
runCommand "microvm-kernel" { inherit linux; } ''
  cp $linux/bzImage "$out"
''
