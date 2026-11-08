{ lib, callPackage }:
let
  files = lib.attrNames (
    lib.filterAttrs (
      name: type: type == "regular" && lib.hasSuffix ".nix" name && name != "default.nix"
    ) (lib.readDir ./.)
  );
  languages = lib.map (
    f:
    let
      lang = callPackage ./${f} { };
    in
    {
      name = lib.removeSuffix ".nix" f;
      inherit (lang)
        label
        package
        sourceFile
        execute
        ;
      packageName = lang.packageName or lang.package.pname;
      aliases = lang.aliases or [ ];
      compile = lang.compile or null;
    }
  ) files;
in
languages
