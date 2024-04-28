{ pkgs ? import <nixpkgs> {} }:

(pkgs.buildFHSEnv {
  name = "simple-wl-env";
  targetPkgs = pkgs: (with pkgs; [
    wayland
    libxkbcommon
  ]);  
  runScript = ''
    bash
  '';
}).env