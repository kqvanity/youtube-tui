with import <nixpkgs> { };
mkShell {
  buildInputs = [
    pkg-config
    openssl
    libsixel
    cmake
    autoconf
    makeWrapper
    automake
    libtool
    xorg.libxcb
    mpv
  ];
  env.PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  env.RUST_BACKTRACE = 1;

}
