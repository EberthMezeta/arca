# Arca

Gestor de contraseñas **100 % local**: sin nube ni cuentas. Todo en **Rust**, con una interfaz nativa en **Slint**
(sin navegador embebido ni JavaScript), en español e inglés. El diseño criptográfico se inspira en Bitwarden.

- [Instalar](#instalar)
- [Uso](#uso)
- [Seguridad](#seguridad)
- [Desarrollo](#desarrollo)
- [Arquitectura](#arquitectura)

## Instalar

En Debian, Ubuntu y derivadas, con el paquete `.deb` (necesita `cargo install --locked cargo-deb`):

```bash
scripts/install-deb.sh   # genera el .deb y lo instala (o reinstala): binario, icono y entrada del menú
sudo apt remove arca     # desinstalar (la bóveda no se toca)
```

## Uso

**Atajos:** `Ctrl K` paleta de comandos · `Ctrl F` buscar · `Ctrl N` nuevo inicio de sesión · `Ctrl S` guardar ·
`Esc` cancelar · `Ctrl L` bloquear · `↑ ↓` moverse por la lista.

**Dónde se guardan los datos** (`~/.local/share/dev.arca.desktop/`):

| Archivo | Contenido |
|---|---|
| `personal.vault` | La bóveda cifrada (y `personal.vault.bak`, la versión anterior) |
| `personal.vault.d/` | Adjuntos cifrados |
| `unlock-throttle.json` | Intentos fallidos de desbloqueo |
| `vault-revision.json` | Última revisión vista (detección de rollback) |
| `arca.lock` | Impide abrir dos instancias a la vez |

> **Antes se llamaba RustVault.** Al arrancar por primera vez, Arca mueve los datos desde
> `~/.local/share/dev.rustvault.desktop/` sin sobrescribir nunca una bóveda que ya exista. Las copias `.rvexport`
> antiguas se siguen importando.

## Seguridad

### Cifrado

| Pieza | Elección |
|---|---|
| Derivación de clave | Argon2id · 256 MiB · 3 iteraciones · 4 hilos · salt aleatoria de 32 B |
| Clave de bóveda | 256 bits aleatorios, envuelta con la clave derivada. Cambiar la maestra **rota la clave** y vuelve a cifrar todo, adjuntos incluidos |
| Cifrado | XChaCha20-Poly1305 con nonce aleatorio de 192 bits en cada guardado |
| Integridad | La cabecera (parámetros y salt) va como dato autenticado: un byte alterado impide abrir la bóveda |
| Subclaves | HKDF-SHA256 por propósito (contenido, adjuntos) y una clave distinta por adjunto |
| Rollback | Un contador cifrado dentro de la bóveda sube en cada guardado; si al abrir trae uno menor que el último visto, se avisa |

### Contraseña maestra

- Mínimo 12 caracteres y puntuación 4 de zxcvbn (más de 10¹⁰ intentos), contando palabras propias de la app y en
  español («arca», «contraseña», «bóveda»…).
- Se exige al crear la bóveda, al cambiar la maestra y en las copias cifradas. Una bóveda antigua con una maestra más
  débil se sigue abriendo, con un aviso para cambiarla.
- Se normaliza a Unicode NFC: «é» compuesto o descompuesto (macOS) abre igual en cualquier sistema.
- Tras 3 intentos fallidos, retardo exponencial (máx. 5 min) que sobrevive a reiniciar la app. También se aplica a
  cambiar la maestra, exportar y guardar adjuntos, que vuelven a pedirla aunque la bóveda esté abierta.

### En uso

- **Interfaz:** Slint nativo en el mismo proceso, sin superficie web. Los secretos llegan enmascarados (también al
  editor), el valor real solo se publica al pulsar «Mostrar» y vuelve a ocultarse a los 30 s.
- **Bloqueo:** por inactividad (medida también con un reloj monótono: atrasar la hora no lo retrasa), al minimizar
  (opcional), al suspender y, en Linux, al bloquear la pantalla. Leer un archivo a importar nunca lo retrasa.
- **Al bloquear** se vacían el editor, los diálogos, los formularios, el generador, lo revelado y cualquier
  exportación pendiente.
- **Portapapeles:** marcado como sensible para que los gestores de historial no lo guarden, y vaciado a los 30 s, al
  bloquear y al cerrar (si sigue conteniendo el secreto).
- **Memoria:** `zeroize` en claves, secretos y texto descifrado, con buffers de tamaño exacto para no dejar copias al
  realocar. Sin volcados de memoria y, en Linux, `PR_SET_DUMPABLE = 0` (otros procesos no pueden leerla).

### Disco y archivos externos

- Escritura atómica (`.tmp` + fsync + rename). La copia `.bak` no se genera al cambiar la maestra ni al borrar para
  siempre, para que lo borrado no siga en ella.
- Archivos `0600` y carpeta de datos `0700`, también para exportaciones y adjuntos descifrados.
- Límites de tamaño antes de leer: importaciones 64 MB, adjuntos 50 MB, bóveda 256 MB. Argon2id acepta como mucho
  1 GiB (y 16 pasadas en una copia `.arca` importada).
- Lo importado pasa las mismas reglas que el editor. Al exportar a CSV, las celdas que empiezan por `=`, `+`, `-` o
  `@` van como texto (evita la *CSV injection*); contraseñas y TOTP van intactos.

### Límites conocidos

- **Texto de la interfaz:** lo que escribes o revelas pasa por cadenas de Slint que no se borran al liberarse. Un
  malware con tu usuario podría encontrar restos (y también podría instalar un keylogger: contra un equipo
  comprometido ningún gestor protege).
- **`zeroize` es best-effort:** el sistema puede haber copiado memoria (registros, swap). Cifrar la swap lo mitiga.
- **Un SSD no sobrescribe al borrar:** el `.bak` y los adjuntos borrados pueden quedar en el disco. Cambiar la maestra
  rota la clave y deja eso ilegible con la contraseña nueva.
- **Rollback:** quien pueda escribir en tu disco también puede retroceder o borrar `vault-revision.json`.
- **Retardo de intentos:** borrar `unlock-throttle.json` con la app cerrada lo reinicia. Frente a quien tiene el
  archivo, lo que protege es el coste de Argon2id.
- **Exportaciones antiguas** siguen cifradas con su contraseña; cambiar la maestra no las afecta.
- **Portapapeles:** no todos los gestores de historial respetan la marca de contenido sensible.
- **Sin auditoría externa.**

### Pendiente

- [ ] Revisión externa del código, aunque sea informal.
- [ ] Algún día: CI que ejecute tests, clippy, `cargo audit` y `cargo deny check` en cada cambio.

## Desarrollo

Dependencias del sistema (una sola vez):

```bash
sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev libdbus-1-dev
```

| Tarea | Comando |
|---|---|
| Ejecutar | `cargo run -p arca` |
| Binario optimizado | `cargo build -p arca --release` → `target/release/arca` |
| Tests (núcleo y controladores) | `cargo test --workspace` |
| Lint | `cargo clippy --workspace --all-targets` |
| Formato (`rustfmt.toml`, 120 columnas) | `cargo fmt --all` |
| Vulnerabilidades conocidas | `cargo audit` |
| Licencias, orígenes y crates sin mantener (`deny.toml`) | `cargo deny check` |

Herramientas extra (una sola vez): `cargo install --locked cargo-deb cargo-audit cargo-deny cargo-fuzz`.

### Paquete `.deb`

```bash
cargo deb -p arca   # compila en release y genera target/debian/arca_<versión>_amd64.deb
```

Qué incluye y dónde va se configura en `[package.metadata.deb]` de `crates/arca-desktop/Cargo.toml`; la entrada del
menú está en `packaging/linux/arca.desktop`. La versión sale del campo `version` de ese `Cargo.toml`.

### Fuzzing

Del lector de `.vault` y de los importadores (necesita nightly):

```bash
cd crates/vault-core
cargo +nightly fuzz run vault_file fuzz/corpus/vault_file fuzz/seeds/vault_file -- -max_total_time=300
cargo +nightly fuzz run bitwarden_json_import fuzz/corpus/bitwarden_json_import fuzz/seeds/bitwarden_json_import
cargo +nightly fuzz run csv_import fuzz/corpus/csv_import fuzz/seeds/csv_import
```

## Arquitectura

```
crates/vault-core/          Núcleo sin interfaz
  domain/                   Entidades, políticas y puertos (traits)
  application/              Casos de uso y la fachada VaultService, repartida en traits por tema
  infrastructure/           Adaptadores: Argon2id, XChaCha20, archivos, import/export
crates/arca-desktop/        Aplicación de escritorio
  ui/                       Interfaz Slint; bridge.slint es el contrato con Rust
  src/presentation/         Un controlador por zona de la interfaz, unidos por un bus de eventos;
                            ports.rs: lo que necesitan del sistema (diálogos, archivos)
  src/platform/             Adaptadores: portapapeles, bloqueo de pantalla, diálogos, endurecimiento
  src/composition_root.rs   Donde se conectan las piezas concretas
  i18n/                     Textos en español e inglés
```

Patrones usados:

| Patrón | Dónde |
|---|---|
| Ports & Adapters | Almacenamiento, portapapeles, reloj, diálogos y archivos detrás de traits; los tests usan dobles en memoria |
| State | `VaultSessionState`, `AppScreen`, `ExportState` |
| Strategy | Generación de contraseñas, KDF, cifrado, criptografía de la copia `.arca`, política de la maestra |
| Command | `AppCommand` (paleta, atajos, Esc) e `ItemCommand`, cuyo `inverse()` es el «Deshacer» |
| Builder | `VaultDependenciesBuilder`, `ConfirmRequest`, `PromptRequest` |
| Registry / tablas | Formatos de import/export, tipos de elemento, secretos enmascarados por tipo, atajos, idiomas |
| Reglas en lista | Política de la maestra y salud de contraseñas |
| Observer | `VaultEvent` desde otros hilos y `AppEvent` entre controladores |
