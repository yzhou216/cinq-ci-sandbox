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
          # with the standard library for Apple silicon on GNU/Linux, which
          # Nixpkgs cannot build there, so that cargo cross-builds for macOS
          rust = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.${pkgs.rustc.version}.minimal.override {
            extensions = [ "clippy" ];
            targets = lib.optionals isLinux [ "aarch64-apple-darwin" ];
          };
          # What cargo needs on GNU/Linux to cross-build for macOS: the SDK
          # that Nixpkgs fetches from Apple, as Apple ships it, to link
          # against with lld from the toolchain
          macosCross = {
            SDKROOT = "${pkgs.apple-sdk.src}";
            CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = "rust-lld";
            CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS = "-C linker-flavor=ld64.lld";
          };

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
              # is checked against its specification
              postInstall = ''
                cp -r target/share "$out"
                desktop-file-validate "$out/share/applications/cinq.desktop"
              '';

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
          // lib.optionalAttrs isDarwin {
            # Application bundle that runs on macOS without Nix: libiconv,
            # the one library that cinq takes from the store, comes from
            # macOS instead, since Nixpkgs builds it from Apple's own source,
            # and the bundle is signed ad hoc, as Apple silicon requires of
            # all code
            app =
              let
                info = pkgs.writeText "Info.plist" (
                  lib.generators.toPlist { escape = true; } {
                    CFBundleExecutable = "cinq";
                    CFBundleIconFile = "cinq";
                    CFBundleIdentifier = "io.yiyuzhou.cinq";
                    CFBundleInfoDictionaryVersion = "6.0";
                    CFBundleName = "cinq";
                    CFBundlePackageType = "APPL";
                    CFBundleShortVersionString = cargo.version;
                    CFBundleVersion = cargo.version;
                    LSMinimumSystemVersion = pkgs.stdenvNoCC.hostPlatform.darwinMinVersion; # Oldest release that Nixpkgs builds for
                    NSHighResolutionCapable = true;
                    NSHumanReadableCopyright = "Copyright (C) 2026 Yiyu Zhou";
                  }
                );
                # Logo that the build draws, from which every size of the icon is rendered
                logo = "${config.packages.cinq}/share/icons/hicolor/scalable/apps/cinq.svg";
              in
              pkgs.runCommandLocal "cinq-app-${cargo.version}"
                {
                  nativeBuildInputs = with pkgs; [
                    cctools
                    python3Packages.icnsutil
                    rcodesign
                    resvg
                  ];
                }
                ''
                  install -m 755 ${config.packages.cinq}/bin/cinq .
                  install_name_tool -change ${lib.getLib pkgs.libiconv}/lib/libiconv.2.dylib \
                    /usr/lib/libiconv.2.dylib cinq
                  # Load commands, none of which may name the store
                  commands=$(otool -l cinq)
                  if grep /nix/store <<<"$commands"; then
                    exit 1
                  fi

                  mkdir icons
                  for size in 16 32 128 256 512; do
                    resvg -w "$size" ${logo} "icons/icon_''${size}x''${size}.png"
                    resvg -w "$((size * 2))" ${logo} "icons/icon_''${size}x''${size}@2x.png"
                  done

                  contents=$out/cinq.app/Contents
                  install -D cinq "$contents/MacOS/cinq"
                  install -D -m 644 ${info} "$contents/Info.plist"
                  mkdir "$contents/Resources"
                  icnsutil compose --toc "$contents/Resources/cinq.icns" icons/*
                  rcodesign sign "$out/cinq.app"
                  # Run once, which macOS allows only when the signature holds and every library loads
                  "$contents/MacOS/cinq" --version
                '';
          }
          // lib.optionalAttrs isLinux {
            # cinq for Apple silicon, which cargo cross-builds, laid out as a
            # package for macOS holds it: the app bundle that the build lays
            # out, with the program copied in, and the share directory
            macos = pkgs.stdenv.mkDerivation {
              pname = "${cargo.name}-macos";
              inherit (cargo) version;
              inherit (config.packages.cinq) src cargoDeps;

              nativeBuildInputs = [
                rust
                pkgs.rustPlatform.cargoSetupHook
                pkgs.scdoc
              ];

              env = macosCross // {
                SOURCE_DATE_EPOCH = toString inputs.self.lastModified;
              };

              buildPhase = ''
                runHook preBuild
                cargo build --frozen --release --target aarch64-apple-darwin
                runHook postBuild
              '';

              installPhase = ''
                runHook preInstall
                mkdir -p "$out/bin"
                cp -r target/Applications target/share "$out"
                cp target/aarch64-apple-darwin/release/cinq "$out/Applications/cinq.app/Contents/MacOS"
                ln -s ../Applications/cinq.app/Contents/MacOS/cinq "$out/bin/cinq"
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
              {
                LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;
              }
              // macosCross
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
