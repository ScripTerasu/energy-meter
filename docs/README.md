# Documentación — energy-meter

Firmware Rust `no_std` (Embassy) para la placa
**Waveshare ESP32-S3-Touch-AMOLED-1.75**.

Esta carpeta reúne el conocimiento de hardware necesario para trabajar en el
firmware sin tener que volver a investigar hojas de datos, drivers del
fabricante ni el pin-out oficial.

## Índice

- [`hardware.md`](hardware.md) — Visión general de la placa y sus componentes.
- [`pinout.md`](pinout.md) — Asignación completa de pines del ESP32-S3.
- [`i2c-bus.md`](i2c-bus.md) — Bus I2C compartido (touch CST9217 + PMU AXP2101).
- [`display.md`](display.md) — Panel AMOLED CO5300 sobre QSPI y framebuffer PSRAM.
- [`buttons.md`](buttons.md) — Botones BOOT y PWR (y por qué son muy distintos).
- [`firmware-map.md`](firmware-map.md) — Cómo se mapea el hardware a los módulos del crate.

## Recursos del fabricante

- Wiki: <https://www.waveshare.com/wiki/ESP32-S3-Touch-AMOLED-1.75>
- Ejemplos: <https://github.com/waveshareteam/ESP32-S3-Touch-AMOLED-1.75>
- Esquemático: [`ESP32-S3-Touch-AMOLED-1.75C-schematic.pdf`](ESP32-S3-Touch-AMOLED-1.75C-schematic.pdf)

> Nota: el agente no dispone de hardware; toda validación se hace con
> `cargo build`. Las pruebas en placa las realiza el usuario.
