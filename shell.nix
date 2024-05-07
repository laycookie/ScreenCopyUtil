let
  pkgs = import <nixpkgs> {};
in
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc

    rust-analyzer
    rustfmt
	clippy

    pkg-config
    openssl

    libxkbcommon
  ];

  env = {
    RUST_BACKTRACE = "full";
  };
}
