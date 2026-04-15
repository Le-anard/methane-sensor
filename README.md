# Methane Gas Detector — ESP32 + Rust

Embedded Systems Lab project: real-time methane detection with OLED display
and audio-visual alarm, written in Rust using the `esp-idf-hal` framework.

---

## Hardware Wiring

### SSD1306 OLED (I2C — 4 pins only)

| OLED Pin | ESP32 Pin       | Notes                    |
|----------|-----------------|--------------------------|
| GND      | GND             |                          |
| VCC      | 3.3 V           | **3.3 V only, not 5 V**  |
| SCL      | GPIO22          | I2C clock                |
| SDA      | GPIO21          | I2C data                 |

> The SSD1306 default I2C address is `0x3C`.  
> If your module uses `0x3D`, change the address in the `I2CDisplayInterface::new_custom_address(i2c, 0x3D)` call in `main.rs`.

### MQ-4 Methane Sensor

| MQ-4 Pin | ESP32 Pin | Notes                                   |
|----------|-----------|-----------------------------------------|
| VCC      | 5 V       | MQ-4 heater needs 5 V                  |
| GND      | GND       |                                         |
| AOUT     | GPIO34    | Analog output — ADC1 CH6 (input-only)  |
| DOUT     | (unused)  | Digital threshold output (not used here)|

> **Warm-up**: Allow ~60 s after power-on for the MQ-4 heater to stabilise.

### Buzzer

| Pin  | ESP32 Pin | Notes                                         |
|------|-----------|-----------------------------------------------|
| +    | GPIO25    | Drive via NPN transistor for louder buzzers   |
| -    | GND       |                                               |

### LED

| Pin    | ESP32 Pin | Notes                         |
|--------|-----------|-------------------------------|
| Anode  | GPIO26    | Add a 330 Ω series resistor   |
| Cathode| GND       |                               |

---


## Thresholds (Configurable in `main.rs`)

| Constant           | Default | Meaning                           |
|--------------------|---------|-----------------------------------|
| `SAFE_THRESHOLD`   | 1500    | ADC raw — below = Safe            |
| `WARNING_THRESHOLD`| 2500    | ADC raw — between = Warning       |
| above WARNING      | —       | Danger — continuous alarm         |
| `PPM_SCALE`        | 0.244   | Multiply raw ADC to get ~ppm      |
| `FILTER_WINDOW`    | 8       | Moving average samples for noise  |
| `LOOP_DELAY_MS`    | 500     | Sensor poll interval (ms)         |

> These thresholds are **relative ADC units**.  
> For accurate ppm readings, perform a two-point calibration in clean air  
> and at a known CH₄ concentration using your specific MQ-4 module.

---

## Alarm Behaviour

| Status  | LED    | Buzzer         | Display             |
|---------|--------|----------------|---------------------|
| Safe    | Off    | Off            | Normal text         |
| Warning | Blink  | Intermittent   | Inverted status bar |
| Danger  | On     | Continuous     | Inverted status bar |

---

## Setup & Build

### 1. Install the ESP Rust toolchain

```bash
cargo install espup
espup install
# Linux/macOS:
. ~/export-esp.sh
# Windows (PowerShell):
. $env:USERPROFILE\export-esp.ps1
```

### 2. Install `ldproxy` (linker helper)

```bash
cargo install ldproxy
```

### 3. Install `espflash` (flashing tool)

```bash
cargo install espflash
```

### 4. Build

```bash
cd methane_detector
cargo build --release
```

### 5. Flash

```bash
# Replace /dev/ttyUSB0 with your port (Windows: COM3, etc.)
espflash flash --monitor target/xtensa-esp32-espidf/release/methane_detector
```

Or use the combined build+flash shortcut:
```bash
cargo espflash flash --release --monitor
```

---

## Serial Monitor Output (example)

```
[Loop 1] Raw=843  Filtered=421  PPM=103  Status=Safe     Peak=843
[Loop 2] Raw=912  Filtered=593  PPM=145  Status=Safe     Peak=912
[Loop 5] Raw=1823 Filtered=1401 PPM=342  Status=Warning  Peak=1823
[Loop 6] Raw=2701 Filtered=2100 PPM=513  Status=Danger   Peak=2701
```

---

## Optional Challenges (from lab spec)

- [ ] Store peak ADC value and timestamp using `esp_idf_svc::nvs` (NVS flash)
- [ ] Add WiFi + HTTP POST notifications via `esp_idf_svc::wifi` + `esp_idf_svc::http`
- [ ] Implement multiple MQ-4 sensors on additional ADC pins
- [ ] Adjustable thresholds via GPIO buttons + on-screen menu

---

## Troubleshooting

| Symptom                        | Likely Cause                                      |
|-------------------------------|---------------------------------------------------|
| OLED blank                    | Wrong I2C address — try `0x3D`                   |
| Constant Danger reading       | MQ-4 not warmed up yet — wait 60 s               |
| Build error: `xtensa-...`     | Xtensa toolchain not installed / env not sourced  |
| Flash error: permission denied| Add user to `dialout` group on Linux              |
