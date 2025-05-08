{
  description = "rust dev";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        devShells.default =
          mkShell rec {
            nativeBuildInputs = [
              pkg-config
              clang-tools
              llvmPackages.clang
              linuxHeaders
              elfutils
              zlib
              glibc_multi
              (rust-bin.nightly.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                ];
              })
            ];
            buildInputs = [
              libbpf
            ];
            hardeningDisable = [
              "zerocallusedregs"
            ];
            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
            shellHook = ''
              export PS1='\[\e[35m\][\w]\[\e[0m\]\$ '
            '';
          };
      }
    );
}
