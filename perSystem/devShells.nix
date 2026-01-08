{
  perSystem = {
    config,
    pkgs,
    ...
  }: {
    devShells.default = with pkgs;
      mkShell {
        packages = [
          cargo
          cmake
          rustc
          pkg-config
          openssl
          zlib
          rust-analyzer
          rustfmt
          libclang
          clippy
          clang-tools
          config.treefmt.build.wrapper
          tdb
        ];
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.libc.dev}/include -I${pkgs.glibc.dev}/include -I${pkgs.libclang.lib}/lib/clang/19/include";
      };
  };
}
