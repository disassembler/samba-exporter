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
          };
          address = lib.mkOption {
            type = lib.types.str;
            default = "0.0.0.0";
          };
        };

        config = lib.mkIf cfg.enable {
          systemd.services.samba-exporter = {
            description = "Samba Prometheus Exporter";
            after = ["network.target" "smbd.service"];
            wantedBy = ["multi-user.target"];
            serviceConfig = {
              ExecStart = "${self.packages.${system}.default}/bin/samba-exporter --port ${toString cfg.port} --listen-address ${cfg.address}";
              User = "root";
              Restart = "always";
            };
          };
        };
      };
    };
  };
}
