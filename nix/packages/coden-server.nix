{
  lib,
  dockerTools,
  stdenvNoCC,
  makeWrapper,
  mkCranePackage,
  coden-copyright-html,
  coden-kernel,
  coden-rootfs,
  firecracker,
}:
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "coden-server";
  unwrapped = mkCranePackage {
    pname = "${finalAttrs.pname}-unwrapped";
    crate = ../../crates/coden-server;
    extraCrates = [
      ../../crates/coden-vm
      ../../crates/coden-agent-api
    ];
    COPYRIGHT_HTML = "${coden-copyright-html}/copyright.html";
    meta.mainProgram = "coden-server";
  };
  inherit (finalAttrs.unwrapped) version meta;
  nativeBuildInputs = [ makeWrapper ];
  dontUnpack = true;
  buildPhase = ''
    mkdir -p "$out/bin"
    makeWrapper "$unwrapped/bin/coden-server" "$out/bin/coden-server" \
      --set-default VMLINUX ${coden-kernel}/kernel \
      --set-default ROOTFS ${coden-rootfs}/rootfs \
      --set-default FIRECRACKER ${lib.getExe' firecracker "firecracker"} \
      --set-default FIRECRACKER_JAILER ${lib.getExe' firecracker "jailer"}
  '';
  passthru.streamLayeredImage = dockerTools.streamLayeredImage {
    name = "coden";
    tag = "latest";
    contents = finalAttrs.unwrapped;
    config = {
      Env = [
        "VMLINUX=${coden-kernel}/kernel"
        "ROOTFS=${coden-rootfs}/rootfs"
        "FIRECRACKER=${lib.getExe' firecracker "firecracker"}"
        "FIRECRACKER_JAILER=${lib.getExe' firecracker "jailer"}"
        "HTTP_ADDR=0.0.0.0:8080"
      ];
      Entrypoint = [ "/bin/coden-server" ];
    };
  };
})
