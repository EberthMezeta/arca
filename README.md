# RustVault

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
| Memoria | `zeroize` en claves y secretos (también la memoria interna de Argon2id y ChaCha20); las claves se rellenan en su sitio y el JSON descifrado se serializa en un buffer de tamaño exacto para no dejar copias al realocar. Al bloquear se descarta todo el estado descifrado |
| Disco | Escritura atómica (`.tmp` + fsync + rename), copia `.bak` (se borra al cambiar la contraseña maestra). Archivos `0600` y directorio de datos `0700`, también para exportaciones y adjuntos descifrados |
| Fuerza bruta | Retardo exponencial tras 3 intentos fallidos, guardado en disco para que reiniciar la app no lo borre (además del coste de Argon2id, que es lo que protege frente a un ataque con el archivo copiado) |
| Frontend | La clave nunca sale de Rust; la UI (también el editor) recibe secretos enmascarados y solo pide el valor real al pulsar «Mostrar» / «Copiar». Al guardar, un secreto enmascarado sin tocar conserva su valor |
| Tauri | CSP estricta, sin plugins `shell`/`fs`/`http`; los diálogos de archivo se abren desde Rust (la UI no envía rutas) |
| Portapapeles | Se marca como sensible para que los gestores de historial no lo guarden, y se vacía a los 30 s (por defecto), al bloquear la bóveda y al cerrar la app, si sigue conteniendo el secreto |
| Bloqueo | Por inactividad, al minimizar (opcional), al suspender el equipo y, en Linux, al bloquear la pantalla (señales de D-Bus del salvapantallas y de logind) |
| Exportación | Las celdas de CSV que empiezan por `=`, `+`, `-` o `@` se exportan como texto (evita la *CSV injection*); contraseña y TOTP se exportan intactos |

Archivos: `~/.local/share/dev.rustvault.desktop/personal.vault`, los adjuntos cifrados en `personal.vault.d/` y el contador de intentos fallidos en `unlock-throttle.json`.

> **Limitaciones honestas:** no hay auditoría externa; un equipo con malware/keylogger puede capturar la contraseña maestra;
> no todos los gestores de portapapeles respetan la marca de contenido sensible; borrar `zeroize` es *best-effort*
> (el compilador o el sistema pueden haber copiado memoria antes); los strings que ya llegaron a JavaScript al pulsar
> «Mostrar» no se pueden borrar de memoria; borrar el `.bak` en un SSD no garantiza que los bloques se sobrescriban;
> las exportaciones antiguas siguen cifradas con la contraseña con la que se crearon.

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
