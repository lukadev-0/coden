{
  lib,
  dockerTools,
  rootfs,
  microvm-kernel,
  firecracker,
  coden-server,
}:
dockerTools.streamLayeredImage {
  name = "coden";
  contents = coden-server;
  extraCommands = ''
    cp ${rootfs} rootfs
    cp ${microvm-kernel.dev}/vmlinux vmlinux
    mkdir tmp
  '';
  config = {
    Env = [
      "VMLINUX=/vmlinux"
      "ROOTFS=/rootfs"
      "FIRECRACKER=${lib.getExe' firecracker "firecracker"}"
      "FIRECRACKER_JAILER=${lib.getExe' firecracker "jailer"}"
    ];
    Entrypoint = [ "/bin/coden-server" ];
  };
}
