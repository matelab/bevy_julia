{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, naersk, nixpkgs, rust-overlay }:
    let
      systems = [
        "aarch64-linux"
        "i686-linux"
        "x86_64-linux"
      ];
      overlays = [ (import rust-overlay) ];
    in
    builtins.foldl'
      (outputs: system:
        let
          pkgs = import nixpkgs {
            inherit overlays system;
          };
          rust-bin = pkgs.rust-bin.rust-nightly;
          naersk-lib = naersk.lib.${system};#.override {
          #cargo = rust-bin;
          #rust = rust-bin;
          # };
          build-deps = with pkgs; [
            lld
            clang
            pkg-config
            makeWrapper
          ];
          runtime-deps = with pkgs; [
            alsa-lib
            udev
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libxcb
            libGL
            vulkan-loader
            vulkan-headers
          ];
        in
        {
          packages.${system}.bevy_julia = naersk-lib.buildPackage {
            pname = "bevy_julia";
            root = ./.;
            buildInputs = runtime-deps;
            nativeBuildInputs = build-deps;
            overrideMain = attrs: {
              fixupPhase = ''
                wrapProgram $out/bin/bevy_julia \
                  --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtime-deps} \
                  --set CARGO_MANIFEST_DIR $out/share/bevy_julia
                mkdir -p $out/share/bevy_julia
                cp -a assets $out/share/bevy_julia'';
            };
          };
          defaultPackage.${system} = self.packages.${system}.bevy_julia;
        }
      )
      { }
      systems;
}
