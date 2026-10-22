# Consumo de memoria: Slint frente a Tauri

Medición de Arca en reposo, después de rehacer la interfaz con Slint (commit `9bfcdcb`).

## Resultado

| | Tauri + React (antes) | Slint (ahora) |
|---|---|---|
| Memoria en reposo (RSS) | **~245 MB** | **~122 MB** |
| Memoria propia del proceso (RssAnon) | — | **~35 MB** |
| Procesos | 3 o más (app + WebKitGTK web + red) | 1 |
| Hilos | — | 23 |
| Binario | — | 27 MB (release) |

La memoria en reposo bajó a **la mitad**, unos 123 MB menos.

## Cómo se midió

- Binario: `cargo build --release -p arca`, en Linux (Wayland, GPU AMD Barcelo, 12 núcleos).
- La app se abrió con un directorio de datos vacío (`XDG_DATA_HOME` temporal): queda en la
  pantalla de crear bóveda, sin tocar nada.
- Se leyó `/proc/<pid>/status` a los 10, 30 y 60 s. `smaps` no se puede leer porque la app
  activa `PR_SET_DUMPABLE = 0`, así que no hay PSS.

| Tiempo | VmRSS | RssAnon | RssFile | CPU acumulada |
|---|---|---|---|---|
| 10 s | 121,7 MB | 35,1 MB | 86,6 MB | 0 s |
| 30 s | 123,3 MB | 35,2 MB | 88,1 MB | 1 s |
| 60 s | 121,9 MB | 37,4 MB | 84,6 MB | 2 s |

La cifra de Tauri (~245 MB) es la que se observó antes con la app en reposo, sumando sus
procesos. No se volvió a medir aquí porque esa versión ya no está en el repositorio.

## Cómo leer las cifras

- **RssFile (~85 MB)** son sobre todo bibliotecas compartidas: drivers de GPU (Mesa), fuentes,
  libc. Otras aplicaciones ya las tienen cargadas, así que el coste real para el sistema es menor.
- **RssAnon (~35 MB)** es la memoria propia de Arca: estado de la interfaz, texturas y la bóveda.
  Es la parte que de verdad cuesta abrir la app.
- Con Tauri, cada ventana arrastraba un motor web completo (WebKitGTK) con su propio proceso
  de contenido y de red. Esa memoria es casi toda privada.

## Pendiente de revisar

- **Pico de 383 MB** (`VmHWM`) entre los 10 y los 30 s, sin interacción. Luego la memoria
  volvió a ~122 MB. Puede ser la compilación de shaders del driver de GPU en el primer dibujado,
  pero no está confirmado.
- **CPU en reposo**: ~1 s de CPU cada 30 s (≈3 %). En reposo debería acercarse a 0; lo más
  probable es una animación continua, como el cursor parpadeando en el campo de contraseña.

## Repetir la medición

```sh
cargo build --release -p arca
XDG_DATA_HOME=$(mktemp -d) target/release/arca &
sleep 30; grep -E 'VmRSS|RssAnon|RssFile|VmHWM' /proc/$!/status
```
