{ busybox }:
let
  sourceFile = "./program.sh";
in
{
  label = "Shell";

  package = busybox;

  inherit sourceFile;
  execute = [
    "sh"
    sourceFile
  ];
}
