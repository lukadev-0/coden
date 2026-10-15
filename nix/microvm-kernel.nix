{ linux_6_18, linuxManualConfig }:
let
  linux = linux_6_18;
in
linuxManualConfig {
  inherit (linux) src version;
  configfile = ./microvm-kernel.config;
}
