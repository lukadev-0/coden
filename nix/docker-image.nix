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
  config = {
    Env = [
      "VMLINUX=${microvm-kernel}"
      "ROOTFS=${rootfs}"
      "FIRECRACKER=${lib.getExe' firecracker "firecracker"}"
      "FIRECRACKER_JAILER=${lib.getExe' firecracker "jailer"}"
      "HTTP_ADDR=0.0.0.0:8080"
    ];
    Entrypoint = [ "/bin/coden-server" ];
  };
}
