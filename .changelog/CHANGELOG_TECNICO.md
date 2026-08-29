# 📋 Registro Técnico de Cambios

> Documento generado automáticamente con cada commit realizado en el proyecto.
> Contiene el detalle técnico completo de cada cambio para el equipo de desarrollo.

---

## feat(core): migración a aplicación 100% nativa en rust y eliminación de node.js

| Campo | Detalle |
|-------|---------|
| **Fecha** | 2026-08-28 21:31:27 |
| **Autor** | David Mendez (david.mendez@courtbetsd.com) |
| **Branch** | main |
| **Tipo** | Refactor / Architecture / Major Release |

### Archivos Modificados

| Archivo | Estado | Descripción del Cambio |
|---------|--------|----------------------|
| `Cargo.toml` | Agregado | Definición del paquete nativo Rust `cleaner` v1.0.0 con dependencias `eframe`, `egui`, `rfd`, `walkdir` y `crossbeam-channel`. |
| `src/main.rs` | Agregado | Punto de entrada del ejecutable nativo y configuración del viewport de ventana (1060x720). |
| `src/app.rs` | Agregado | Interfaz gráfica inmediata con `egui`, diseño de tarjetas estructuradas, búsqueda en tiempo real, filtros por categoría, revelado en Finder y confirmación de borrado. |
| `src/scanner.rs` | Agregado | Motor multihilo concurrente de recorrido de carpetas, cálculo exacto de tamaños, categorización de objetivos y borrado seguro. |
| `.gitignore` | Modificado | Configuración para ignorar `/target`, `.env` y temporales de Rust. |
| `README.md` | Modificado | Documentación completa de la versión en Rust puro, comandos de ejecución y arquitectura. |
| `CONTRIBUTING.md` | Modificado | Actualización de directrices hacia el estándar de estilo en Rust (`cargo fmt`, `cargo clippy`). |
| `node_modules/` | Eliminado | Remoción de todas las dependencias del ecosistema Node.js. |
| `package.json` | Eliminado | Remoción del manifiesto de npm/pnpm. |
| `pnpm-lock.yaml` | Eliminado | Remoción del archivo de bloqueo de pnpm. |
| `pnpm-workspace.yaml` | Eliminado | Remoción de la configuración de workspace de pnpm. |
| `index.html` | Eliminado | Remoción del frontend web HTML. |
| `renderer.js` | Eliminado | Remoción del código frontend en JavaScript. |
| `style.css` | Eliminado | Remoción de estilos CSS web. |
| `src-tauri/` | Eliminado | Remoción del envoltorio y configuración de Tauri. |

### Detalle Técnico
- **Reemplazo Arquitectónico Total**: Se descartó la arquitectura híbrida Webview/Tauri/Node.js en favor de una aplicación de escritorio nativa compilada directamente a código máquina en Rust mediante `egui` y `eframe` v0.29.
- **Rendimiento y Huella de Memoria**: El binario ejecutable único pesa 6.0 MB (`target/release/cleaner`), consume menos de 30 MB de RAM y arranca de manera instantánea a 60 FPS sin sobrecarga de motor web ni Chromium.
- **Concurrencia sin Bloqueo**: Se diseñó una arquitectura de mensajería con canales `crossbeam-channel` entre el hilo de la UI y los hilos de trabajo en segundo plano para escaneo y eliminación.
- **Inspección y Claridad UI**: Se sustituyó el esquema de tabla estrecha por tarjetas informativas que destacan el proyecto, la ruta relativa, la ruta absoluta completa seleccionable, botón "Reveal in Finder" para inspección en macOS Finder y distintivo de borrado inequívoco (`WILL DELETE: [folder]/`).
- **Eliminación de Glifos Rotos**: Se prescindió de emojis incompatibles con las fuentes del sistema en `egui`, empleando etiquetas estilizadas con color y símbolos estándar.

### Fragmentos de Código Relevantes

```rust
// Motor de escaneo concurrente con omisión de directorios hijos
if entry.depth() > 0 && entry.file_type().is_dir() {
    let name = entry.file_name().to_string_lossy().to_string();
    if target_names.contains(&name) {
        let path = entry.path().to_path_buf();
        let size = get_dir_size(&path);
        let _ = sender.send(ScanMessage::Found(ScanItem { ... }));
        it.skip_current_dir();
    }
}
```

---
