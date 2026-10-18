{
  linux_6_18,
  linuxManualConfig,
  runCommand,
}:
let
  linux = linuxManualConfig {
    inherit (linux_6_18) src version;
    configfile = ./microvm-kernel.config;
  };
in
runCommand "microvm-kernel" { } ''
  cp ${linux.dev}/vmlinux "$out"
''
