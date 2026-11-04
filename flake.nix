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
          crate = (lib.importTOML ./Cargo.toml).package;
          # Date of the latest commit, which the manual pages and packages
          # give rather than the fixed date that builds take otherwise
          sourceDateEpoch = toString inputs.self.lastModified;
          # Rustfmt with the unstable settings of rustfmt.toml turned on
          rustfmt = pkgs.rustfmt.override { asNightly = true; };
          # Rust's own toolchain of the release that Nixpkgs builds rustc
          # from, with the standard library for Apple silicon, which Nixpkgs
          # cannot build on GNU/Linux
          rust = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.${pkgs.rustc.version}.minimal.override {
            targets = [ "aarch64-apple-darwin" ];
          };
          # Moves the program into the app bundle that builds for Darwin lay
          # out, and links it back from bin
          installBundle = ''
            cp -r target/Applications "$out"
            mv "$out/bin/cinq" "$out/Applications/cinq.app/Contents/MacOS"
            ln -s ../Applications/cinq.app/Contents/MacOS/cinq "$out/bin"
          '';
          # glibc of Nixpkgs, which cinq links against, so that other
          # distributions need this one or a newer one
          glibc = lib.versions.majorMinor pkgs.glibc.version;
          # cinq for other distributions of GNU/Linux, laid out under usr,
          # which loads the system's loader and libraries rather than those
          # of the Nix store
          portable =
            let
              directory = if pkgs.stdenv.hostPlatform.isx86_64 then "lib64" else "lib";
              loader = "/${directory}/${baseNameOf pkgs.stdenv.cc.bintools.dynamicLinker}";
            in
            pkgs.runCommand "${crate.name}-portable" { nativeBuildInputs = [ pkgs.patchelf ]; } ''
              mkdir -p "$out/usr"
              cp -r ${config.packages.cinq}/{bin,share} "$out/usr"
              chmod -R u+w "$out/usr"
              patchelf --set-interpreter ${loader} --remove-rpath "$out/usr/bin/cinq"
            '';
          # Package of portable cinq in the format of nfpm's `packager`, which
          # depends on glibc, ALSA, and the libraries that cinq loads, apart
          # from those of the GPU, without which it draws in software
          nfpm =
            packager:
            pkgs.runCommand "${crate.name}-${packager}"
              {
                nativeBuildInputs = [ pkgs.nfpm ];
                env.SOURCE_DATE_EPOCH = sourceDateEpoch;
                nfpmConfig = builtins.toJSON rec {
                  inherit (crate)
                    name
                    version
                    description
                    license
                    ;
                  arch = pkgs.stdenv.hostPlatform.go.GOARCH;
                  maintainer = "Yiyu Zhou <yiyu@yiyuzhou.io>";
                  section = "sound";
                  archlinux.packager = maintainer;
                  # Files alone, leaving their directories to the system
                  contents = [
                    {
                      src = "usr/";
                      dst = "/usr";
                    }
                  ];
                  overrides = {
                    deb.depends = [
                      "libc6 (>= ${glibc})"
                      "libasound2t64 | libasound2"
                      "libdbus-1-3"
                      "libwayland-client0"
                      "libxkbcommon0"
                    ];
                    rpm.depends = [
                      "glibc >= ${glibc}"
                      "alsa-lib"
                      "dbus-libs"
                      "libwayland-client"
                      "libxkbcommon"
                    ];
                    archlinux.depends = [
                      "glibc"
                      "alsa-lib"
                      "dbus"
                      "wayland"
                      "libxkbcommon"
                    ];
                  };
                };
                passAsFile = [ "nfpmConfig" ];
              }
              ''
                cp -r ${portable}/usr usr
                chmod -R u+w usr
                mkdir "$out"
                nfpm package --config "$nfpmConfigPath" --packager ${packager} --target "$out"
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
              pname = crate.name;
              inherit (crate) version;

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

              env.SOURCE_DATE_EPOCH = sourceDateEpoch;

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
                inherit (crate) description;
                license = lib.licenses.gpl3Plus;
                mainProgram = "cinq";
              };
            };
          }
          // lib.optionalAttrs isLinux {
            # cinq for Apple silicon, laid out as the package for Darwin is,
            # which cargo cross-builds against the SDK that Nixpkgs fetches
            # from Apple, linked by lld from the toolchain
            aarch64-darwin-cross = pkgs.stdenv.mkDerivation {
              pname = "${crate.name}-aarch64-darwin";
              inherit (crate) version;
              inherit (config.packages.cinq) src cargoDeps;

              nativeBuildInputs = [
                rust
              ]
              ++ (with pkgs; [
                rustPlatform.cargoSetupHook
                scdoc
              ]);

              env = {
                SDKROOT = "${pkgs.apple-sdk.src}";
                CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = "rust-lld";
                SOURCE_DATE_EPOCH = sourceDateEpoch;
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

            deb = nfpm "deb";
            rpm = nfpm "rpm";
            archlinux = nfpm "archlinux";
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ config.packages.cinq ];

            packages = [
              rustfmt
            ]
            ++ (with pkgs; [
              clippy
              reuse
              rust-analyzer
            ]);

            env = lib.optionalAttrs isLinux {
              LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;
            };
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
                inherit (crate) edition;
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
