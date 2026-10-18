{
  lib,
  dockerTools,
  rootfs,
  microvm-kernel,
  firecracker,
  coden-server-unwrapped,
}:
dockerTools.streamLayeredImage {
  name = "coden";
  contents = coden-server-unwrapped;
  extraCommands = ''
    mkdir tmp
  '';
  config = {
    Env = [
      "VMLINUX=${microvm-kernel}"
      "ROOTFS=${rootfs}"
      "FIRECRACKER=${lib.getExe' firecracker "firecracker"}"
      "FIRECRACKER_JAILER=${lib.getExe' firecracker "jailer"}"
    ];
    Entrypoint = [ "/bin/coden-server" ];
  };
}
