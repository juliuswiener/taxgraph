#!/usr/bin/env bash
# Install the Catala/Clerk toolchain into a dedicated opam switch "taxgraph".
# Tested on Ubuntu 24.04. On Arch, replace the apt line with:
#   sudo pacman -S --needed opam m4 base-devel pkgconf gmp
set -u
export OPAMYES=1
export OPAMCONFIRMLEVEL=unsafe-yes

echo "=== system dependencies ==="
sudo apt-get update
sudo apt-get install -y opam m4 build-essential pkg-config libgmp-dev

echo "=== opam init ==="
opam init --bare -y

echo "=== opam switch create taxgraph 4.14.2 ==="
opam switch create taxgraph 4.14.2 || true
eval "$(opam env --switch=taxgraph --set-switch)"
ocaml --version

echo "=== opam install catala ==="
opam install -y catala
eval "$(opam env --switch=taxgraph --set-switch)"

echo "=== versions ==="
catala --version 2>&1 || echo "CATALA MISSING"
clerk --version 2>&1 || echo "CLERK MISSING"
command -v catala clerk
echo "=== done ==="
