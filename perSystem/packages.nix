{inputs, ...}: {
  perSystem = {
    system,
    config,
    lib,
    pkgs,
    ...
  }: {
    packages = rec {
      samba-exporter = let
        naersk-lib = inputs.naersk.lib.${system};
      in
        naersk-lib.buildPackage rec {
          pname = "samba-exporter";

          src = with lib.fileset;
            toSource {
              root = ./..;
              fileset = unions [
                ../Cargo.lock
                ../Cargo.toml
                ../src
              ];
            };
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.libc.dev}/include -I${pkgs.glibc.dev}/include -I${pkgs.libclang.lib}/lib/clang/19/include";

          buildInputs = with pkgs; [
            pkg-config
            tdb
          ];

          meta = {
            mainProgram = pname;
            maintainers = with lib.maintainers; [
              disassembler
            ];
            license = with lib.licenses; [
              asl20
            ];
          };
        };
      default = samba-exporter;
    };
  };
}
