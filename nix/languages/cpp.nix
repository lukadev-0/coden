{ gcc }:
let
  sourceFile = "./program.c";
  outFile = "./program";
in
{
  label = "C++";
  aliases = [
    "c++"
    "cxx"
  ];

  package = gcc;
  packageName = "gcc";

  inherit sourceFile;
  compile = [
    "g++"
    "-std=c++26"
    "-o"
    outFile
    sourceFile
  ];

  execute = [ outFile ];
}
