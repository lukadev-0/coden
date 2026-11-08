{ python3 }:
let
  sourceFile = "./program.py";
in
{
  label = "Python";
  aliases = [ "py" ];

  packageName = "python";
  package = python3;

  inherit sourceFile;
  execute = [
    "python"
    sourceFile
  ];
}
