# Pin-out del ESP32-S3

Asignación de pines derivada del `pin_config.h` oficial de Waveshare y del
esquemático de la placa. **Antes de reclamar un GPIO para una función nueva,
revisa esta tabla para no colisionar con periféricos existentes.**

## Display AMOLED CO5300 (QSPI)

| Señal        | GPIO   | Usado en firmware |
| ------------ | ------ | ----------------- |
| SCLK         | GPIO38 | ✅ `display`      |
| CS           | GPIO12 | ✅ `display`      |
| D0 / SIO0    | GPIO4  | ✅ `display`      |
| D1 / SIO1    | GPIO5  | ✅ `display`      |
| D2 / SIO2    | GPIO6  | ✅ `display`      |
| D3 / SIO3    | GPIO7  | ✅ `display`      |
| LCD_RESET    | GPIO39 | ✅ `display`      |

Periféricos internos que consume el display: `SPI2`, `DMA_CH0`, `PSRAM`.

## Bus I2C compartido (touch + PMU + otros)

| Señal | GPIO   | Usado en firmware |
| ----- | ------ | ----------------- |
| SDA   | GPIO15 | ✅ `i2c`          |
| SCL   | GPIO14 | ✅ `i2c`          |

Periférico interno: `I2C0`. Ver [`i2c-bus.md`](i2c-bus.md) para los dispositivos.

## Touch CST9217 (pines dedicados, aparte del I2C)

| Señal    | GPIO   | Nivel activo | Usado en firmware |
| -------- | ------ | ------------ | ----------------- |
| TP_INT   | GPIO11 | Bajo         | ✅ `touch`        |
| TP_RESET | GPIO40 | Bajo (reset) | ✅ `touch`        |

> ⚠️ **GPIO40 (TP_RESET) es el reset del touch**, distinto del **GPIO39
> (LCD_RESET)** que usa el display. No confundirlos.

## Botones

| Botón | GPIO / bus     | Usado en firmware |
| ----- | -------------- | ----------------- |
| BOOT  | GPIO0          | ✅ `main`         |
| PWR   | AXP2101 (I2C)  | ✅ `power`        |

El botón PWR **no es un GPIO**: está en la tecla de encendido del PMU. Ver
[`buttons.md`](buttons.md).

## microSD (SPI) — no usado por el firmware

| Señal      | GPIO   |
| ---------- | ------ |
| CS (SS)    | GPIO41 |
| DI (MOSI)  | GPIO1  |
| DO (MISO)  | GPIO3  |
| SCK (SCLK) | GPIO2  |

## Resumen de GPIOs ocupados

```
GPIO0  → BOOT button
GPIO1  → SD MOSI            (libre para el firmware actual)
GPIO2  → SD SCLK            (libre para el firmware actual)
GPIO3  → SD MISO            (libre para el firmware actual)
GPIO4  → LCD D0
GPIO5  → LCD D1
GPIO6  → LCD D2
GPIO7  → LCD D3
GPIO11 → TP_INT (touch)
GPIO12 → LCD CS
GPIO14 → I2C SCL
GPIO15 → I2C SDA
GPIO38 → LCD SCLK
GPIO39 → LCD_RESET
GPIO40 → TP_RESET (touch)
GPIO41 → SD CS             (libre para el firmware actual)
```

Al añadir nuevas funciones, prefiere GPIOs que no aparezcan arriba y que estén
expuestos en el header de 8 pines de la placa.
