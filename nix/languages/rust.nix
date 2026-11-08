{ rustc }:
let
  sourceFile = "./program.rs";
  outFile = "./program";
in
{
  label = "Rust";
  aliases = [ "rs" ];

  package = rustc;

  inherit sourceFile;
  compile = [
    "rustc"
    "-o"
    outFile
    sourceFile
  ];
  execute = [ outFile ];
}
