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
    openTelemetry = {
      enable = lib.mkEnableOption "OpenTelemetry exporter for Coden server";
      serviceName = lib.mkOption {
        type = lib.types.str;
        default = "coden";
      };
      endpoint = lib.mkOption {
        type = lib.types.str;
        default =
          if cfg.openTelemetry.protocol == "grpc" then "http://localhost:4317" else "http://localhost:4318";
      };
      protocol = lib.mkOption {
        type = lib.types.enum [
          "grpc"
          "http/protobuf"
        ];
        default = "grpc";
      };
    };
    environment = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.nullOr (
          lib.types.oneOf [
            lib.types.str
            lib.types.path
            lib.types.package
          ]
        )
      );
      default = { };
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
      }
      // lib.optionalAttrs (cfg.openTelemetry.enable) {
        CODEN_OTEL = "1";
        OTEL_SERVICE_NAME = cfg.openTelemetry.serviceName;
        OTEL_EXPORTER_OTLP_ENDPOINT = cfg.openTelemetry.endpoint;
        OTEL_EXPORTER_OTLP_PROTOCOL = cfg.openTelemetry.protocol;
      }
      // cfg.environment;
      script = ''
        CODEN_JAILER_UID=$(id -u coden) \
        CODEN_JAILER_GID=$(id -g coden) \
        exec ${lib.getExe pkgs.coden-server}
      '';
    };
  };
}
