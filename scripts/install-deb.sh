#!/usr/bin/env bash
# Genera el .deb de Arca y lo instala (o reinstala) en este equipo.
#
# El paquete se copia antes a /tmp: apt lee los paquetes con el usuario sin privilegios
# `_apt`, que no puede entrar en la carpeta personal, y sin la copia avisa de que lo lee como root.
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

package_path="$(cargo deb -p arca | tail -n 1)"
temporary_package="/tmp/$(basename "$package_path")"
cp "$package_path" "$temporary_package"

sudo apt install --reinstall "$temporary_package"
rm -f "$temporary_package"
