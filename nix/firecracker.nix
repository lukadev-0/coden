{ stdenvNoCC, fetchurl }:
let
  hashes = {
    x86_64-linux = "sha256-BglKEQiunoKqTCOndaqSdY9T8RddQiJw2dYWLLmt5Vg=";
  };
  arch = stdenvNoCC.hostPlatform.parsed.cpu.name;
in
stdenvNoCC.mkDerivation (
  finalAttrs:
  let
    inherit (finalAttrs) version;
  in
  {
    pname = "firecracker";
    version = "1.17.0";
    src = fetchurl {
      url = "https://github.com/firecracker-microvm/firecracker/releases/download/v${version}/firecracker-v${version}-${arch}.tgz";
      hash = hashes.${stdenvNoCC.hostPlatform.system};
    };
    installPhase = ''
      mkdir -p "$out/bin"
      cp firecracker-v${version}-${arch} "$out/bin/firecracker"
      cp jailer-v${version}-${arch} "$out/bin/jailer"
    '';
    meta.mainProgram = "firecracker";
  }
)
