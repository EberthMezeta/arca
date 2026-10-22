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

## Revisión del pico y de la CPU (22-10-2026)

Se repitió la medición con el binario release, muestreando `/proc/<pid>/status` cada 0,2 s
durante 30 s y la CPU por hilo (`/proc/<pid>/task/*/stat`).

- **Pico de 383 MB: no se reproduce.** `VmHWM` se quedó en 113–128 MB en dos ejecuciones, una de
  ellas con `MESA_SHADER_CACHE_DISABLE=true` (como si fuera el primer arranque), así que no lo
  causa la compilación de shaders.
- **El pico es Argon2id.** Una derivación con los parámetros reales (`KeyDerivationParameters::RECOMMENDED`,
  la misma `Argon2idKeyDerivation` de la app) en un proceso aparte sube `VmHWM` de 1,9 MB a
  263,5 MB en 0,47 s y `VmRSS` vuelve a 2 MB al terminar. 122 MB en reposo + 261,6 MB = 383 MB:
  la medición original coincidió con crear o desbloquear la bóveda. Es el coste buscado del
  KDF (256 MiB), dura menos de medio segundo y se libera.
- **CPU en reposo: ~0–0,4 %, no 3 %.** 1 tick (10 ms) en 30 s con la ventana sin foco; ~40 ms
  cada 10 s en otra ejecución, casi todo en el hilo principal: el cursor parpadeando obliga a
  redibujar dos veces por segundo (~2 ms cada vez). La cifra anterior salía de la columna «CPU
  acumulada» con resolución de 1 s, que no sirve para medir algo tan pequeño.

Conclusión: no hay nada que corregir en reposo. Si se quiere vigilar, medir con ticks de
`/proc/<pid>/stat` (resolución de 10 ms), no con segundos enteros.

## Repetir la medición

```sh
cargo build --release -p arca
XDG_DATA_HOME=$(mktemp -d) target/release/arca &
sleep 30; grep -E 'VmRSS|RssAnon|RssFile|VmHWM' /proc/$!/status
awk '{print "CPU (ticks de 10 ms):", $14 + $15}' /proc/$!/stat
```
