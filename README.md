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
| Disco | Escritura atómica (`.tmp` + fsync + rename) y copia `.bak` (no se genera al cambiar la contraseña maestra, al eliminar definitivamente ni al vaciar la papelera, y la anterior se borra: lo borrado no sigue en la copia). Archivos `0600` y directorio de datos `0700`, también para exportaciones y adjuntos descifrados (en un USB FAT/exFAT no hay permisos) |
| Fuerza bruta | Retardo exponencial tras 3 intentos fallidos (máx. 5 min), guardado en disco para que reiniciar la app no lo borre. Se aplica también a cambiar la contraseña y a exportar. Frente a un ataque con el archivo copiado protege el coste de Argon2id |
| Reautenticación | Exportar y guardar un adjunto descifrado piden la contraseña maestra aunque la bóveda esté desbloqueada |
| Contraseña | Se normaliza a Unicode NFC antes de Argon2id: «é» compuesto o descompuesto (macOS) abre igual en cualquier sistema. Las bóvedas y copias creadas antes de normalizar se siguen abriendo |
| Frontend | La clave nunca sale de Rust; la UI (también el editor) recibe secretos enmascarados y solo pide el valor real al pulsar «Mostrar» / «Copiar»; lo revelado se vuelve a ocultar a los 30 s. Al guardar, un secreto enmascarado sin tocar conserva su valor |
| Tauri | CSP estricta, sin plugins `shell`/`fs`/`http`; los diálogos de archivo se abren desde Rust (la UI no envía rutas) |
| Portapapeles | Se marca como sensible para que los gestores de historial no lo guarden, y se vacía a los 30 s (por defecto), al bloquear la bóveda y al cerrar la app, si sigue conteniendo el secreto |
| Bloqueo | Por inactividad, al minimizar (opcional), al suspender el equipo y, en Linux, al bloquear la pantalla (señales de D-Bus del salvapantallas y de logind, solo de la sesión propia) |
| Exportación | Las celdas de CSV que empiezan por `=`, `+`, `-` o `@` se exportan como texto (evita la *CSV injection*); contraseña y TOTP se exportan intactos |
| Archivos externos | Importaciones (máx. 64 MB) y adjuntos (máx. 50 MB) se rechazan antes de leerlos si son más grandes; lo importado pasa las mismas reglas que el editor. Argon2id acepta como mucho 1 GiB al abrir un archivo |
| Proceso | Sin volcados de memoria; en Linux, `PR_SET_DUMPABLE = 0` impide leer la memoria con `ptrace` desde otros procesos. Una sola instancia a la vez (dos se pisarían los cambios). El bloqueo funciona aunque un fallo interno haya dejado el estado inconsistente |

Archivos: `~/.local/share/dev.arca.desktop/personal.vault`, los adjuntos cifrados en `personal.vault.d/`, el contador de intentos fallidos en `unlock-throttle.json` y el bloqueo de instancia única en `arca.lock`.

> **Antes se llamaba RustVault.** Al arrancar por primera vez, Arca mueve la bóveda, los adjuntos y las preferencias desde
> `~/.local/share/dev.rustvault.desktop/` (nunca sobrescribe una bóveda que ya exista en la carpeta nueva). Las copias
> `.rvexport` antiguas se siguen importando: el formato no cambió.

### Límites conocidos

Lo que Arca **no** puede garantizar, y por qué:

- **Memoria del WebView.** La UI corre en WebKit, un proceso aparte. Lo que escribes en la pantalla de desbloqueo y lo
  que revelas con «Mostrar» pasa por JavaScript, donde no se puede borrar de memoria (lo libera el recolector de
  basura cuando quiere), y ese proceso no queda protegido por `PR_SET_DUMPABLE`. Se reduce el tiempo de exposición
  (lo revelado se oculta a los 30 s y los diálogos vacían su estado al cerrarse), pero un malware que corra con tu
  usuario podría leerlo. Ese mismo malware podría también instalar un keylogger: contra un equipo comprometido
  ningún gestor de contraseñas protege.
- **`zeroize` es best-effort.** El compilador o el sistema pueden haber copiado memoria (registros, swap) antes de
  borrarla. Cifrar la swap del sistema lo mitiga.
- **Borrar en un SSD no sobrescribe.** El `.bak` y los adjuntos borrados pueden seguir en bloques libres del disco,
  y los adjuntos se descifran con la clave de bóveda actual. Si te preocupa, cambia la contraseña maestra: rota la
  clave y lo que quede en el disco deja de poder abrirse con la contraseña nueva (no con la antigua).
- **Exportaciones antiguas.** Siguen cifradas con la contraseña con la que se crearon; cambiar la maestra no las afecta.
- **Rollback.** Quien pueda escribir en tu disco puede sustituir la bóveda por una versión anterior suya y la app no
  lo detecta.
- **Retardo de intentos.** Borrar `unlock-throttle.json` con la app cerrada reinicia el contador. Es intencionado: el
  retardo solo frena a quien tiene la app delante; frente a quien tiene acceso al disco protege el coste de Argon2id.
- **Portapapeles.** No todos los gestores de historial respetan la marca de contenido sensible.
- **Sin auditoría externa.** El diseño se ha revisado internamente, pero nadie independiente ha auditado el código.

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
