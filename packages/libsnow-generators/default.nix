{ cargo
, makeWrapper
, openssl
, pkg-config
, rustc
, rustPlatform
, sqlite
}:

rustPlatform.buildRustPackage rec {
  pname = "libsnow-generators";
  version = "0.0.1";

  src = [ ../.. ];

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  nativeBuildInputs = [
    cargo
    makeWrapper
    pkg-config
    rustc
    rustPlatform.cargoSetupHook
  ];

  buildInputs = [
    openssl
    sqlite
  ];
}
