# SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
#
# SPDX-License-Identifier: GPL-3.0-or-later

{
  description = "CINQ Is Not QLab, a minimal cue-based show controller";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, nixpkgs, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.treefmt-nix.flakeModule ];
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          inherit (pkgs.stdenvNoCC.hostPlatform) isDarwin isFreeBSD isLinux;
          cargo = (lib.importTOML ./Cargo.toml).package;
          # Rustfmt with the unstable settings of rustfmt.toml turned on
          rustfmt = pkgs.rustfmt.override { asNightly = true; };
          # Rust's own build of the release that Nixpkgs builds rustc from,
          # with the standard library for Apple silicon, which Nixpkgs cannot
          # build on GNU/Linux, for cargo to cross-build for Darwin there
          rust = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.${pkgs.rustc.version}.minimal.override {
            extensions = [ "clippy" ];
            targets = lib.optionals isLinux [ "aarch64-apple-darwin" ];
          };
          # Settings with which cargo cross-builds for Apple silicon: the SDK
          # that Nixpkgs fetches from Apple, unchanged, linked against with
          # lld from the toolchain
          crossEnv = {
            SDKROOT = "${pkgs.apple-sdk.src}";
            CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = "rust-lld";
            CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS = "-C linker-flavor=ld64.lld";
          };
          # Moves the program into the app bundle that builds for Darwin lay
          # out, and links it back from bin
          installBundle = ''
            cp -r target/Applications "$out"
            mv "$out/bin/cinq" "$out/Applications/cinq.app/Contents/MacOS"
            ln -s ../Applications/cinq.app/Contents/MacOS/cinq "$out/bin"
          '';

          # Libraries that cinq loads at run time rather than links: those
          # of winit and wgpu for Wayland alone, and libdbus, through which
          # rfd reaches the file chooser portal
          runtimeLibs = lib.optionals isLinux (
            with pkgs;
            [
              dbus
              libGL
              libxkbcommon
              vulkan-loader
              wayland
            ]
          );
        in
        {
          packages = {
            default = config.packages.cinq;

            cinq = pkgs.rustPlatform.buildRustPackage {
              pname = cargo.name;
              inherit (cargo) version;

              src = lib.fileset.toSource {
                root = ./.;
                fileset = lib.fileset.unions [
                  ./.cargo
                  ./Cargo.toml
                  ./Cargo.lock
                  ./build.rs
                  ./src
                  ./man
                  ./share
                ];
              };

              cargoLock.lockFile = ./Cargo.lock;

              nativeBuildInputs = with pkgs; [
                pkg-config
                scdoc
                desktop-file-utils
              ];

              # ALSA, which cpal plays sound through on GNU/Linux and FreeBSD
              buildInputs = lib.optionals (isLinux || isFreeBSD) [ pkgs.alsa-lib ];

              # Manual pages dated by the latest commit rather than by the
              # fixed date that builds take otherwise
              env.SOURCE_DATE_EPOCH = toString inputs.self.lastModified;

              # Share directory that the build lays out, whose desktop entry
              # is checked against its specification, and on Darwin the app
              # bundle
              postInstall = ''
                cp -r target/share "$out"
                desktop-file-validate "$out/share/applications/cinq.desktop"
              ''
              + lib.optionalString isDarwin installBundle;

              postFixup = lib.optionalString isLinux ''
                patchelf --add-rpath ${lib.makeLibraryPath runtimeLibs} "$out/bin/cinq"
              '';

              meta = {
                inherit (cargo) description;
                license = lib.licenses.gpl3Plus;
                mainProgram = "cinq";
              };
            };
          }
          // lib.optionalAttrs isLinux {
            # cinq for Apple silicon, which cargo cross-builds, laid out as
            # the package for Darwin is
            aarch64-darwin-cross = pkgs.stdenv.mkDerivation {
              pname = "${cargo.name}-aarch64-darwin";
              inherit (cargo) version;
              inherit (config.packages.cinq) src cargoDeps;

              nativeBuildInputs = [
                rust
                pkgs.rustPlatform.cargoSetupHook
                pkgs.scdoc
              ];

              env = crossEnv // {
                SOURCE_DATE_EPOCH = toString inputs.self.lastModified;
              };

              buildPhase = ''
                runHook preBuild
                cargo build --frozen --release --target aarch64-apple-darwin
                runHook postBuild
              '';

              installPhase = ''
                runHook preInstall
                install -D target/aarch64-apple-darwin/release/cinq "$out/bin/cinq"
                cp -r target/share "$out"
                ${installBundle}
                runHook postInstall
              '';
            };
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ config.packages.cinq ];

            # Toolchain first, ahead of the cargo and rustc of the package
            packages = [
              rust
              pkgs.reuse
              rustfmt
              pkgs.rust-analyzer
            ];

            env = lib.optionalAttrs isLinux (
              crossEnv // { LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs; }
            );
          };

          # What nix flake check holds the repository to, along with the
          # formatting below: the build of cinq, which runs its tests, and
          # the REUSE specification for every file
          checks = {
            cinq = config.packages.cinq;
            reuse = pkgs.runCommandLocal "reuse-lint" { nativeBuildInputs = [ pkgs.reuse ]; } ''
              reuse --root ${./.} lint
              touch "$out"
            '';
          };

          # Formatters of every kind of file in the repository that has one,
          # which nix fmt runs, and which nix flake check holds the files to
          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              rumdl-format.enable = true;
              rustfmt = {
                enable = true;
                package = rustfmt;
                inherit (cargo) edition;
              };
              taplo.enable = true;
              xmllint.enable = true;
              yamlfmt = {
                enable = true;
                settings.formatter.retain_line_breaks_single = true; # Blank line after the license
              };
            };
          };
        };
    };
}
