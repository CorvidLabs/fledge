{
  description = "Fledge — one CLI, your whole dev lifecycle";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "fledge";
          version = "1.7.0";
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          postInstall = ''
            installShellCompletion --cmd fledge \
              --bash <($out/bin/fledge completions bash) \
              --zsh <($out/bin/fledge completions zsh) \
              --fish <($out/bin/fledge completions fish)
          '';

          meta = with pkgs.lib; {
            description = "One CLI, your whole dev lifecycle";
            homepage = "https://github.com/CorvidLabs/fledge";
            license = licenses.mit;
            mainProgram = "fledge";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];
        };
      });
}
