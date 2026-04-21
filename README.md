# Methane Gas Detector 
Real-time methane detection system using ESP32-C6, OLED display, audio-visual alarm (LED indicator & buzzer) & embedded Rust using the `esp-idf-hal` standard library for easier going.

## Hardware Wiring

### SSD1306 OLED (I2C — 4 pins only)

| OLED Pin | ESP32 Pin       | Notes                    |
|----------|-----------------|--------------------------|
| GND      | GND             |                          |
| VCC      | 3.3 V           | **3.3 V only, not 5 V**  |
| SCL      | GPIO0           | I2C clock                |
| SDA      | GPIO1           | I2C data                 |

### MQ-4 Methane Sensor

| MQ-4 Pin | ESP32 Pin | Notes                                   |
|----------|-----------|-----------------------------------------|
| VCC      | 5 V       | MQ-4 heater needs 5 V                   |
| GND      | GND       |                                         |
| AOUT     | GPIO44    | Analog output — ADC1 CH6 (input-only)   |
| DOUT     | (unused)  | Digital threshold output (not used here)|

> **Warm-up**: Allow 60 s after power-on for the MQ-4 heater to stabilise.

### Buzzer/Alarm

| Pin  | ESP32 Pin | Notes                                         |
|------|-----------|-----------------------------------------------|
| +    | GPIO25    | Drive via NPN transistor for louder buzzers   |
| -    | GND       |                                               |

### INDICATOR LED

| Pin    | ESP32 Pin | Notes                                     |
|--------|-----------|-------------------------------            |
| Anode  | GPIO15     | with a 330Ω pullup series resistor for    |
| Cathode| GND       |                                           |      

### Illustrative image
<img width="958" height="882" alt="Screenshot From 2026-04-15 18-39-40 (Copy)" src="https://github.com/user-attachments/assets/54edc312-5f95-470d-8cf4-5d38dba37a35" />

## Variables
1. R0 - established internal baseline resistance of the sensor when placed in fresh/clean air.
2. Rs - gas sensor's current resistance.
3. Rl - The load resistance (in ohms) populated physically on your sensor module board (usually between 1k-47k ohms).
4. Vref - refference voltage; 5v, from the microcontroller.
5. Vout - The actual analog voltage (in Volts) that the sensor sends to your MCU's analog pin.
6. 1012.7 - he scaling factor (a) derived from the slope of the MQ-4 datasheet's log-log sensitivity curve.
7. 4095 - maximum count of a 12-bit ADC.

## Thresholds.

| Constant           | Default   | Meaning & alarm beharviour.                                        |
|--------------------|-----------|--------------------------------------------------------------------|
| `SAFE_ZONE`        | MGC = 1500 |Safe methane gas levels; far below danger zone.                    |
| `WARNING ⚠️ ⚠️ ⚠️` | MGC = 2500 |Levels nearing danger zone; like 30% distance from dangerous level.|
| `DANGER 🚫 🚫 🚫`  | MGC = —    |Levels having reached harzadous.                                   |
| `PPM_SCALE`        | 0.244     | Multiply raw ADC to get ~ppm.                                      |
| `FILTER_WINDOW`    | 8         | Moving average samples for noise.                                  |
| `LOOP_DELAY_MS`    | 500       | Sensor poll interval (ms).                                         |

> These MGC (gas concentration values in ppm) are calculated from 12-bit-ADC values from the MQ-4 sensor as follows:
--- GC = 1012.7{(Rl*(4095-ADC))/(ADC*R0)}^(-2.786) .

## Alarm Behaviour.

| Status       | LED     | Buzzer             | Display                                                          |
|--------------|---------|--------------------|------------------------------------------------------------------|
| Safe zone    | Off     | Off                | MGC value: _. Status: Normal 🌿🌿🌿. Coment: Relax ✅.           |
| Warning      | Blinks  | Intervaled beeping | MGC value: _. Status: Aproaching danger ⚠️⚠️⚠️. Coment: Vacate 🏃🏾|
| Danger zone  | On      | Continuous pitch   | MGC value: _. Status: HAZZARD ❗❗❗. Coment: VACATE FAST 🏃🏾🏃🏾🏃🏾 |


## Important to do's.

### 2. Install `ldproxy` (linker helper)
cargo install ldproxy
### 3. Install `espflash` (flashing tool)
cargo install espflash
### 4. Build
cargo build --release
### 5. Flash
Or use the combined build+flash shortcut:
cargo espflash flash --release --monitor
## Serial Monitor Output (example)

[Loop 1] Raw=843  Filtered=421  PPM=103  Status=Safe     Peak=843
[Loop 2] Raw=912  Filtered=593  PPM=145  Status=Safe     Peak=912
[Loop 5] Raw=1823 Filtered=1401 PPM=342  Status=Warning  Peak=1823
[Loop 6] Raw=2701 Filtered=2100 PPM=513  Status=Danger   Peak=2701
