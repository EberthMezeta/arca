#!/usr/bin/env bash
# Genera el .deb de Arca y lo instala (o reinstala) en este equipo.
#
# El paquete se copia antes a /tmp: apt lee los paquetes con el usuario sin privilegios
# `_apt`, que no puede entrar en la carpeta personal, y sin la copia avisa de que lo lee como root.
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

if ! cargo deb --version >/dev/null 2>&1; then
    echo "Falta cargo-deb, la herramienta que genera el .deb. Instálala con:" >&2
    echo "    cargo install cargo-deb" >&2
    exit 1
fi

package_path="$(cargo deb -p arca | tail -n 1)"
temporary_package="/tmp/$(basename "$package_path")"
# La copia se borra al salir, también si apt falla o se cancela: el script no deja nada en /tmp.
trap 'rm -f "$temporary_package"' EXIT
cp "$package_path" "$temporary_package"

# --reinstall: sirve igual para instalar la primera vez que para volver a instalar la misma versión.
sudo apt install --reinstall "$temporary_package"
