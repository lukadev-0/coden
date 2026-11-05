{
  lib,
  pkgs,
  config,
  ...
}:
let
  cfg = config.services.coden;
in
{
  options.services.coden = {
    enable = lib.mkEnableOption "Coden server";
    package = lib.mkPackageOption pkgs "coden-server" { };
    httpAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1:31543";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.coden = {
      description = "Coden VM";
      isSystemUser = true;
      group = "coden";
    };
    users.groups.coden = { };

    systemd.services.coden = {
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        StateDirectory = "coden";
        Restart = "on-failure";
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        ProtectClock = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
      };
      environment = {
        HTTP_ADDR = cfg.httpAddress;
      };
      script = ''
        CODEN_JAILER_UID=$(id -u coden) \
        CODEN_JAILER_GID=$(id -g coden) \
        exec ${lib.getExe pkgs.coden-server}
      '';
    };
  };
}
