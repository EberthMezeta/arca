# Arca

Gestor de contraseñas **100 % local** (sin nube ni cuentas) con un diseño criptográfico inspirado en Bitwarden.
Backend en **Rust**, escritorio con **Tauri 2** e interfaz en **React + TypeScript** (español / inglés).

## Puesta en marcha (Linux)

```bash
# 1. Dependencias del sistema para Tauri (una sola vez)
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev

# 2. Dependencias JS
pnpm install

# 3. Desarrollo
pnpm tauri dev

# 4. Instalable (.deb / .AppImage / .rpm en target/release/bundle)
pnpm tauri build
```

Tests:

```bash
cargo test -p vault-core          # núcleo: cripto, formato, casos de uso, import/export
cargo clippy --workspace --all-targets
pnpm test                         # frontend (Vitest)
pnpm typecheck
```

## Seguridad

| Pieza | Elección |
|---|---|
| Derivación de clave | Argon2id · 256 MiB · 3 iteraciones · 4 hilos · salt aleatoria de 32 B |
| Clave de bóveda | 256 bits aleatorios, envuelta con la clave derivada. Cambiar la maestra **rota la clave** y vuelve a cifrar todo (adjuntos incluidos): una copia vieja del archivo + la contraseña antigua no abre las versiones nuevas |
| Cifrado | XChaCha20-Poly1305 con nonce aleatorio de 192 bits en cada guardado |
| Integridad | La cabecera (parámetros KDF, salt) va como AAD: cualquier byte alterado impide abrir la bóveda |
| Subclaves | HKDF-SHA256 por propósito (contenido, adjuntos); una clave distinta por adjunto |
| Memoria | `zeroize` en claves y secretos (también la memoria interna de Argon2id y ChaCha20) y en títulos, usuarios, URLs y datos personales al bloquear; las claves se rellenan en su sitio y el JSON descifrado, las frases de paso y los archivos leídos usan buffers de tamaño exacto para no dejar copias al realocar |
| Disco | Escritura atómica (`.tmp` + fsync + rename) y copia `.bak` (no se genera al cambiar la contraseña maestra y la anterior se borra). Archivos `0600` y directorio de datos `0700`, también para exportaciones y adjuntos descifrados (en un USB FAT/exFAT no hay permisos) |
| Fuerza bruta | Retardo exponencial tras 3 intentos fallidos (máx. 5 min), guardado en disco para que reiniciar la app no lo borre. Se aplica también a cambiar la contraseña y a exportar. Frente a un ataque con el archivo copiado protege el coste de Argon2id |
| Reautenticación | Exportar pide la contraseña maestra aunque la bóveda esté desbloqueada |
| Frontend | La clave nunca sale de Rust; la UI (también el editor) recibe secretos enmascarados y solo pide el valor real al pulsar «Mostrar» / «Copiar». Al guardar, un secreto enmascarado sin tocar conserva su valor |
| Tauri | CSP estricta, sin plugins `shell`/`fs`/`http`; los diálogos de archivo se abren desde Rust (la UI no envía rutas) |
| Portapapeles | Se marca como sensible para que los gestores de historial no lo guarden, y se vacía a los 30 s (por defecto), al bloquear la bóveda y al cerrar la app, si sigue conteniendo el secreto |
| Bloqueo | Por inactividad, al minimizar (opcional), al suspender el equipo y, en Linux, al bloquear la pantalla (señales de D-Bus del salvapantallas y de logind, solo de la sesión propia) |
| Exportación | Las celdas de CSV que empiezan por `=`, `+`, `-` o `@` se exportan como texto (evita la *CSV injection*); contraseña y TOTP se exportan intactos |
| Archivos externos | Importaciones (máx. 64 MB) y adjuntos (máx. 50 MB) se rechazan antes de leerlos si son más grandes; lo importado pasa las mismas reglas que el editor. Argon2id acepta como mucho 1 GiB al abrir un archivo |
| Proceso | Sin volcados de memoria; en Linux, `PR_SET_DUMPABLE = 0` impide leer la memoria con `ptrace` desde otros procesos. Una sola instancia a la vez (dos se pisarían los cambios) |

Archivos: `~/.local/share/dev.arca.desktop/personal.vault`, los adjuntos cifrados en `personal.vault.d/`, el contador de intentos fallidos en `unlock-throttle.json` y el bloqueo de instancia única en `arca.lock`.

> **Antes se llamaba RustVault.** Al arrancar por primera vez, Arca mueve la bóveda, los adjuntos y las preferencias desde
> `~/.local/share/dev.rustvault.desktop/` (nunca sobrescribe una bóveda que ya exista en la carpeta nueva). Las copias
> `.rvexport` antiguas se siguen importando: el formato no cambió.

> **Limitaciones honestas:** no hay auditoría externa; un equipo con malware/keylogger puede capturar la contraseña maestra;
> no todos los gestores de portapapeles respetan la marca de contenido sensible; borrar `zeroize` es *best-effort*
> (el compilador o el sistema pueden haber copiado memoria antes); los strings que ya llegaron a JavaScript al pulsar
> «Mostrar» no se pueden borrar de memoria; borrar el `.bak` en un SSD no garantiza que los bloques se sobrescriban;
> las exportaciones antiguas siguen cifradas con la contraseña con la que se crearon; quien pueda escribir en tu disco
> puede sustituir la bóveda por una versión anterior suya sin que la app lo detecte (*rollback*), y borrar
> `unlock-throttle.json` reinicia el retardo (no protege frente a quien ya tiene acceso al disco).

## Arquitectura

```
crates/vault-core/        Núcleo sin Tauri (testeable de forma aislada)
  domain/                 Entidades, valores, políticas y puertos (traits)
  application/            Casos de uso + fachada VaultService
  infrastructure/         Adaptadores: Argon2id, XChaCha20, ficheros, import/export
src-tauri/                Capa de presentación: comandos finos, eventos, raíz de composición
src/                      React: domain → infrastructure (vaultApiClient) → application (stores/hooks) → presentation
```

Patrones usados para evitar cadenas de `if`:

- **State**: `VaultSessionState { Locked, Unlocked }` en Rust y `SCREEN_COMPONENTS` en React.
- **Strategy**: generación de contraseñas, KDF y cifrado detrás de traits.
- **Registry**: formatos de import/export (`TransferFormatRegistry`), tipos de elemento (`ITEM_CONTENT_TYPE_REGISTRY`), secciones de la UI.
- **Reglas en lista**: política de contraseña maestra y salud de contraseñas (`PasswordHealthRule`).
- **Repository / Ports & Adapters**: almacenamiento, portapapeles y reloj inyectados → tests con dobles en memoria.
- **Observer**: eventos `vault-locked` y `clipboard-cleared` del backend a la UI.

## Atajos de teclado

`Ctrl K` paleta de comandos · `Ctrl F` buscar · `Ctrl N` nuevo inicio de sesión · `Ctrl S` guardar · `Esc` cancelar · `Ctrl L` bloquear · `↑ ↓` moverse por la lista
