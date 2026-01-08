{
  flake = {config, ...}: {
    nixosModules = let
      name = "prometheus-samba-exporter";
    in {
      default = {
        imports = [config.nixosModules.${name}];
      };

      ${name} = {
        self,
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.${name};
        system = pkgs.stdenv.hostPlatform.system;
      in {
        options.services.samba-exporter = {
          enable = lib.mkEnableOption "Samba Prometheus Exporter";

          port = lib.mkOption {
            type = lib.types.port;
            default = 9922;
            description = "Port to listen on.";
          };

          address = lib.mkOption {
            type = lib.types.str;
            default = "0.0.0.0";
            description = "Address to listen on.";
          };

          sambaPackage = lib.mkOption {
            type = lib.types.package;
            default = pkgs.samba;
            description = "The samba package to use for smbstatus.";
          };

          clusterMode = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether to parse PIDs in cluster mode (node:pid).";
          };

          extraArgs = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [];
            description = "Extra command line arguments for the exporter.";
          };
        };

        config = lib.mkIf cfg.enable {
          # Add the exporter to the system path for manual debugging
          environment.systemPackages = [self.packages.${pkgs.system}.default];

          systemd.services.samba-exporter = {
            description = "Samba Prometheus Exporter";
            after = ["network.target" "samba.service"];
            wantedBy = ["multi-user.target"];

            serviceConfig = {
              # Use the absolute path to smbstatus from the specific Nix store path
              ExecStart =
                lib.concatStringsSep " " [
                  "${self.packages.${pkgs.system}.default}/bin/samba-exporter"
                  "--listen-address ${cfg.address}"
                  "--port ${toString cfg.port}"
                  "--smbstatus-path ${cfg.sambaPackage}/bin/smbstatus"
                  (lib.optionalString cfg.clusterMode "--cluster-mode")
                ]
                + " "
                + (lib.concatStringsSep " " cfg.extraArgs);

              # Permissions: smbstatus needs to read /var/lib/samba/*.tdb
              # process.rs needs to read /proc/[pid]/io and /proc/[pid]/fd
              User = "root";
              Group = "root";

              # Hardening
              CapabilityBoundingSet = ["CAP_DAC_READ_SEARCH" "CAP_NET_BIND_SERVICE"];
              DevicePolicy = "closed";
              NoNewPrivileges = true;
              ProtectSystem = "full";
              ProtectHome = "read-only";
              Restart = "always";
              RestartSec = "5s";
            };
          };
        };
      };
    };
  };
}
