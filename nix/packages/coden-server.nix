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
  nodejs,
  pnpm_11,
  pnpmConfigHook,
  fetchPnpmDeps,
}:
let
  pnpm = pnpm_11;

  frontend = stdenvNoCC.mkDerivation (finalAttrs: {
    pname = "coden-frontend";
    version = "0.0.0";
    src = lib.sources.cleanSource ../../frontend;

    nativeBuildInputs = [
      nodejs
      pnpm
      pnpmConfigHook
    ];

    pnpmDeps = fetchPnpmDeps {
      inherit (finalAttrs) pname version src;
      inherit pnpm;
      fetcherVersion = 5;
      hash = "sha256-mh+8p7/wB6oTcnInt1eRh0plAhkOnWGv9mc2q6DvCiw=";
    };

    buildPhase = ''
      pnpm run build
    '';

    installPhase = ''
      cp -r build "$out"
    '';
  });
in
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

    inherit frontend;
    preBuild = ''
      mkdir -p "frontend"
      ln -s "$frontend" "frontend/build"
    '';

    meta.mainProgram = "coden-server";
  };
  inherit (finalAttrs.unwrapped) version meta;
  nativeBuildInputs = [ makeWrapper ];
  dontUnpack = true;
  buildPhase = ''
    mkdir -p "$out/bin"
    makeWrapper "$unwrapped/bin/coden-server" "$out/bin/coden-server" \
      --set-default CODEN_KERNEL ${coden-kernel}/kernel \
      --set-default CODEN_ROOTFS ${coden-rootfs}/rootfs \
      --set-default CODEN_LANGUAGES_MANIFEST ${coden-rootfs}/languages.json \
      --set-default FIRECRACKER ${lib.getExe' firecracker "firecracker"} \
      --set-default FIRECRACKER_JAILER ${lib.getExe' firecracker "jailer"}
  '';
  passthru.streamLayeredImage = dockerTools.streamLayeredImage {
    name = "coden";
    tag = "latest";
    contents = finalAttrs.unwrapped;
    config = {
      Env = [
        "CODEN_KERNEL=${coden-kernel}/kernel"
        "CODEN_ROOTFS=${coden-rootfs}/rootfs"
        "CODEN_LANGUAGES_MANIFEST=${coden-rootfs}/languages.json"
        "FIRECRACKER=${lib.getExe' firecracker "firecracker"}"
        "FIRECRACKER_JAILER=${lib.getExe' firecracker "jailer"}"
        "HTTP_ADDR=0.0.0.0:8080"
      ];
      Entrypoint = [ "/bin/coden-server" ];
    };
  };
})
