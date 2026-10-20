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
| Clave de bóveda | 256 bits aleatorios, envuelta con la clave derivada (cambiar la maestra no re-cifra nada) |
| Cifrado | XChaCha20-Poly1305 con nonce aleatorio de 192 bits en cada guardado |
| Integridad | La cabecera (parámetros KDF, salt) va como AAD: cualquier byte alterado impide abrir la bóveda |
| Subclaves | HKDF-SHA256 por propósito (contenido, adjuntos); una clave distinta por adjunto |
| Memoria | `zeroize` en claves y secretos; al bloquear se descarta todo el estado descifrado |
| Disco | Escritura atómica (`.tmp` + fsync + rename), copia `.bak`, permisos `0600` |
| Fuerza bruta | Retardo exponencial tras 3 intentos fallidos (además del coste de Argon2id) |
| Frontend | La clave nunca sale de Rust; la UI recibe secretos enmascarados y solo pide el valor real al pulsar «Mostrar» / «Copiar» |
| Tauri | CSP estricta, sin plugins `shell`/`fs`/`http`; los diálogos de archivo se abren desde Rust (la UI no envía rutas) |
| Portapapeles | Se vacía automáticamente (30 s por defecto) si sigue conteniendo el secreto |

Archivos: `~/.local/share/dev.rustvault.desktop/personal.vault` y los adjuntos cifrados en `personal.vault.d/`.

> **Limitaciones honestas:** no hay auditoría externa; un equipo con malware/keylogger puede capturar la contraseña maestra;
> algunos gestores de portapapeles del escritorio guardan historial por su cuenta.

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
