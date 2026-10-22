# Arca

Gestor de contraseñas **100 % local** (sin nube ni cuentas) con un diseño criptográfico inspirado en Bitwarden.
Todo en **Rust**: núcleo criptográfico y una interfaz nativa con **Slint** (español / inglés), sin navegador embebido
ni JavaScript.

## Puesta en marcha (Linux)

```bash
# 1. Dependencias del sistema (una sola vez)
sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev libdbus-1-dev

# 2. Desarrollo
cargo run -p arca

# 3. Binario optimizado (target/release/arca)
cargo build -p arca --release
```

Tests:

```bash
cargo test --workspace            # núcleo (cripto, formato, casos de uso, import/export) y presentación
cargo clippy --workspace --all-targets
```

Fuzzing del lector de `.vault` y de los importadores (necesita nightly y `cargo install --locked cargo-fuzz`):

```bash
cd crates/vault-core
cargo +nightly fuzz run vault_file fuzz/corpus/vault_file fuzz/seeds/vault_file -- -max_total_time=300
cargo +nightly fuzz run bitwarden_json_import fuzz/corpus/bitwarden_json_import fuzz/seeds/bitwarden_json_import
cargo +nightly fuzz run csv_import fuzz/corpus/csv_import fuzz/seeds/csv_import
```

Dependencias (`cargo install --locked cargo-audit cargo-deny`, una sola vez):

```bash
cargo audit                       # vulnerabilidades conocidas (RustSec)
cargo deny check                  # vulnerabilidades, crates sin mantener o retirados, licencias y orígenes (deny.toml)
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
| Rollback | Cada guardado sube un contador que va cifrado y autenticado dentro de la bóveda; la app apunta aparte el último que vio (`vault-revision.json`). Si al desbloquear el archivo trae uno menor, avisa: alguien lo sustituyó por una versión anterior (o restauraste una copia) |
| Reautenticación | Exportar y guardar un adjunto descifrado piden la contraseña maestra aunque la bóveda esté desbloqueada |
| Contraseña maestra | Mínimo 12 caracteres y puntuación 4 de zxcvbn (más de 10¹⁰ intentos), con palabras propias de la app y en español («arca», «contraseña», «bóveda»…) que su diccionario inglés no conoce. Se exige al crear la bóveda, al cambiarla y en las copias cifradas; una bóveda antigua con una maestra más débil se sigue abriendo |
| Contraseña | Se normaliza a Unicode NFC antes de Argon2id: «é» compuesto o descompuesto (macOS) abre igual en cualquier sistema. Las bóvedas y copias creadas antes de normalizar se siguen abriendo |
| Interfaz | Slint nativo en el mismo proceso: sin WebView, sin JavaScript y sin superficie web (ni CSP que configurar). La interfaz solo lee propiedades y emite callbacks; los controladores Rust son los únicos que tocan la bóveda. Los secretos llegan enmascarados (también al editor) y el valor real solo se publica al pulsar «Mostrar»; lo revelado se vuelve a ocultar a los 30 s. Al guardar, un secreto enmascarado sin tocar conserva su valor |
| Campos sensibles | Las contraseñas maestras se vacían del campo en cuanto Rust las lee; al bloquear se vacían el editor, los diálogos, los formularios de ajustes, el generador y los detalles revelados |
| Portapapeles | Se marca como sensible para que los gestores de historial no lo guarden, y se vacía a los 30 s (por defecto), al bloquear la bóveda y al cerrar la app, si sigue conteniendo el secreto |
| Bloqueo | Por inactividad, al minimizar (opcional), al suspender el equipo y, en Linux, al bloquear la pantalla (señales de D-Bus del salvapantallas y de logind, solo de la sesión propia) |
| Exportación | Las celdas de CSV que empiezan por `=`, `+`, `-` o `@` se exportan como texto (evita la *CSV injection*); contraseña y TOTP se exportan intactos |
| Archivos externos | Importaciones (máx. 64 MB) y adjuntos (máx. 50 MB) se rechazan antes de leerlos si son más grandes; lo importado pasa las mismas reglas que el editor. Argon2id acepta como mucho 1 GiB al abrir un archivo |
| Proceso | Sin volcados de memoria; en Linux, `PR_SET_DUMPABLE = 0` impide leer la memoria con `ptrace` desde otros procesos. Una sola instancia a la vez (dos se pisarían los cambios). El bloqueo funciona aunque un fallo interno haya dejado el estado inconsistente |

Archivos: `~/.local/share/dev.arca.desktop/personal.vault`, los adjuntos cifrados en `personal.vault.d/`, el contador de intentos fallidos en `unlock-throttle.json`, la última revisión vista de la bóveda en `vault-revision.json` y el bloqueo de instancia única en `arca.lock`.

> **Antes se llamaba RustVault.** Al arrancar por primera vez, Arca mueve la bóveda, los adjuntos y las preferencias desde
> `~/.local/share/dev.rustvault.desktop/` (nunca sobrescribe una bóveda que ya exista en la carpeta nueva). Las copias
> `.rvexport` antiguas se siguen importando: el formato no cambió.

### Límites conocidos

Lo que Arca **no** puede garantizar, y por qué:

- **Texto de la interfaz.** Lo que escribes y lo que revelas con «Mostrar» pasa por cadenas de Slint (`SharedString`),
  que se liberan sin sobrescribirse, y el campo de texto puede guardar copias propias (historial de deshacer). Al ir
  todo en el mismo proceso, ya no hay un proceso aparte sin protección: `PR_SET_DUMPABLE` cubre también la interfaz.
  Se reduce además el tiempo de exposición (los campos se vacían al leerlos y al bloquear, lo revelado se oculta a los
  30 s), pero un malware que corra con tu usuario y privilegios para leer memoria podría encontrar restos. Ese mismo
  malware podría también instalar un keylogger: contra un equipo comprometido ningún gestor de contraseñas protege.
- **`zeroize` es best-effort.** El compilador o el sistema pueden haber copiado memoria (registros, swap) antes de
  borrarla. Cifrar la swap del sistema lo mitiga.
- **Borrar en un SSD no sobrescribe.** El `.bak` y los adjuntos borrados pueden seguir en bloques libres del disco,
  y los adjuntos se descifran con la clave de bóveda actual. Si te preocupa, cambia la contraseña maestra: rota la
  clave y lo que quede en el disco deja de poder abrirse con la contraseña nueva (no con la antigua).
- **Exportaciones antiguas.** Siguen cifradas con la contraseña con la que se crearon; cambiar la maestra no las afecta.
- **Rollback.** Se detecta si solo se sustituye la bóveda, pero quien pueda escribir en tu disco puede retroceder
  también `vault-revision.json` (o borrarlo) y la app no lo notará: no hay ningún almacén de confianza fuera del disco.
  Tras avisar una vez, la versión abierta pasa a ser la de referencia.
- **Retardo de intentos.** Borrar `unlock-throttle.json` con la app cerrada reinicia el contador. Es intencionado: el
  retardo solo frena a quien tiene la app delante; frente a quien tiene acceso al disco protege el coste de Argon2id.
- **Portapapeles.** No todos los gestores de historial respetan la marca de contenido sensible.
- **Sin auditoría externa.** El diseño se ha revisado internamente, pero nadie independiente ha auditado el código.

### Pendiente de seguridad

Ordenado por impacto frente a esfuerzo:

- [x] **Auditar dependencias**: `cargo audit` y `cargo deny check` (`deny.toml`) pasan sin vulnerabilidades.
      Las excepciones son crates sin mantener que trae Slint y no tocan la bóveda. De momento se ejecutan a mano.
- [x] **Fuzzing** del lector del archivo de bóveda y de los importadores JSON y CSV (`crates/vault-core/fuzz`):
      3 min por objetivo sin fallos. La copia cifrada `.arca` no tiene objetivo: cada intento exige contraseña y un Argon2id de hasta 1 GiB.
- [x] **Detectar rollback**: contador de revisión dentro de la bóveda y última revisión vista aparte (ver «Límites»).
- [x] **Revisar la política de contraseña maestra**: subida de puntuación 3 a 4 de zxcvbn (con 3 pasaban
      `Barcelona1992!` o `arcaarca2024!`) y palabras de contexto en español. La interfaz lee la política del núcleo.
- [ ] **Revisión externa** del código, aunque sea informal.

Algún día: **CI** que ejecute tests, clippy, `cargo audit` y `cargo deny check` en cada cambio (ahora no hay; se
lanzan a mano).

### Pendiente de código

- [x] **Quitar la duplicación del editor adaptable** (`item_editor.slint`): carpeta/etiquetas y valor/«oculto» de los
      campos personalizados están escritos dos veces (en fila y apilados). Extraerlos a componentes pequeños.
- [x] **Contrato Slint ↔ Rust con tipos, no cadenas**: `Workspace.shortcut("save")` y
      `reveal-sealed("descriptor:" + índice)` dependen de texto que el compilador no comprueba. Pasar a enums de Slint
      (`enum Shortcut { … }`) y callbacks con parámetros tipados.
- [x] **Atajos como tabla**: `presentation/shortcuts.rs` (tecla → `Shortcut` → etiqueta para la paleta).
- [x] **Apariencia de `Button` por variante**: el `background` encadena cuatro ternarios; moverlo a una función de
      apariencia como `ToastAppearance`.
- [x] **Dividir los controladores grandes**: `item_details_controller.rs` (≈600 líneas) y `workspace_controller.rs`
      (≈550) en piezas por responsabilidad (adjuntos, TOTP, filas de detalle…).
- [x] **Atribución de Slint**: se usa con la licencia royalty-free, que exige mostrar la atribución (p. ej. el widget
      `AboutSlint` en «Acerca de») o elegir otra licencia (GPL-3.0 o comercial).
- [x] **Memoria y CPU** (ver `docs/consumo-memoria.md`): el pico de 383 MB es Argon2id (+261,6 MB durante
      ~0,5 s al crear o desbloquear, medido) y la CPU en reposo es ~0–0,4 %; el 3 % salía de medir con resolución de 1 s.

## Arquitectura

```
crates/vault-core/        Núcleo sin interfaz (testeable de forma aislada)
  domain/                 Entidades, valores, políticas y puertos (traits)
  application/            Casos de uso + fachada VaultService
  infrastructure/         Adaptadores: Argon2id, XChaCha20, ficheros, import/export
crates/arca-desktop/      Aplicación de escritorio
  ui/                     Interfaz Slint; bridge.slint es el contrato (globals) con Rust
  src/presentation/       Un controlador por zona de la interfaz, conectados por un bus de eventos
  src/platform/           Portapapeles, bloqueo de pantalla, diálogos de archivo, instancia única, endurecimiento
  src/composition_root.rs Raíz de composición
  i18n/                   Catálogos es / en
```

Patrones usados para evitar cadenas de `if`:

- **State**: `VaultSessionState { Locked, Unlocked }` en el núcleo y `AppScreen` en la interfaz.
- **Strategy**: generación de contraseñas, KDF y cifrado detrás de traits.
- **Registry**: formatos de import/export (`TransferFormatRegistry`), tipos de elemento (`ITEM_KIND_DEFINITIONS`), idiomas (`SUPPORTED_LANGUAGES`).
- **Reglas en lista**: política de contraseña maestra y salud de contraseñas (`PasswordHealthRule`).
- **Repository / Ports & Adapters**: almacenamiento, portapapeles y reloj inyectados → tests con dobles en memoria.
- **Observer**: `VaultEvent` (bloqueo, portapapeles vaciado) desde otros hilos, y `AppEvent` entre controladores.

## Atajos de teclado

`Ctrl K` paleta de comandos · `Ctrl F` buscar · `Ctrl N` nuevo inicio de sesión · `Ctrl S` guardar · `Esc` cancelar · `Ctrl L` bloquear · `↑ ↓` moverse por la lista
