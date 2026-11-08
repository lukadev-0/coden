{ gcc }:
let
  sourceFile = "./program.c";
  outFile = "./program";
in
{
  label = "C";

  package = gcc;
  packageName = "gcc";

  inherit sourceFile;
  compile = [
    "gcc"
    "-o"
    outFile
    sourceFile
  ];

  execute = [ outFile ];
}
