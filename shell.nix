{ pkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/nixos-22.11.tar.gz";
    sha256 = "1xi53rlslcprybsvrmipm69ypd3g3hr7wkxvzc73ag8296yclyll";
  }) {}
}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    openssl
    openssl.dev
    pkg-config
  ];
  
  shellHook = ''
    export OPENSSL_DIR="${pkgs.openssl.dev}"
    export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
    export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
  '';
}
