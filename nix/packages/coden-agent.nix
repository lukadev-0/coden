{ mkCranePackage }:
mkCranePackage {
  crate = ../../crates/coden-agent;
  extraCrates = [ ../../crates/coden-agent-api ];
}
