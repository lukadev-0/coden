{
  lib,
  callPackage,
  runCommand,
  writeClosure,
  buildEnv,
  squashfsTools,
  jq,
  busybox,
  additionalPackages ? [ ],
}:
let
  languages = callPackage ../languages { };
  languagePackages = lib.map (lang: lang.package) languages;
  languagesManifest = lib.map (lang: {
    inherit (lang)
      label
      name
      aliases
      compile
      execute
      ;
    source_file = lang.sourceFile;
    package = {
      name = lang.packageName;
      inherit (lang.package) version;
    };
  }) languages;

  envPackages = [
    busybox
  ]
  ++ languagePackages
  ++ additionalPackages;

  rootfsEnv = buildEnv {
    name = "rootfs-env";
    paths = envPackages;
    pathsToLink = [
      "/bin"
      "/etc"
    ];
  };
in
runCommand "rootfs"
  {
    inherit rootfsEnv languagesManifest;
    __structuredAttrs = true;
    unsafeDiscardReferences.out = true;
    closure = writeClosure envPackages;
    nativeBuildInputs = [
      squashfsTools
      jq
    ];
  }
  ''
    sourceFiles=()

    addFile() {
      sourceFiles+=("$1")
      echo "add $path"
    }

    while read -r path; do
      addFile "$path"
    done < "$closure"

    cd "$rootfsEnv"
    for path in *; do
      addFile "$path"
    done

    mkdir -p "$out"
    jq --compact-output .languagesManifest "$NIX_ATTRS_JSON_FILE" > "$out/languages.json"
    mksquashfs "''${sourceFiles[@]}" "$out/rootfs" \
      -no-strip -force-uid 0 -force-gid 0 -repro -info -pseudo-override \
      -comp zstd -Xcompression-level 15 \
      -root-mode 755 \
      -p "/root d 700 0 0" \
      -p "/runner d 700 1000 1000" \
      -p "/dev d 755 0 0" \
      -p "/mnt d 755 0 0" \
      -p "/proc d 555 0 0" \
      -p "/sys d 555 0 0" \
      -p "/tmp d 1777 0 0"
  ''
