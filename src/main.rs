use esp_idf_svc::hal::{
    adc::{self, oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver}},
    delay::FreeRtos,
    gpio::PinDriver,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::FromValueType,
};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X18_BOLD, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

use log::info;

// ── Tuneable constants ──────────────────────────────────────────────────────

const SAFE_THRESHOLD: u16 = 1500; 
const WARNING_THRESHOLD: u16 = 2500; 

const PPM_SCALE: f32 = 0.244; 
const FILTER_WINDOW: usize = 8;
const LOOP_DELAY_MS: u32 = 500;

// ── Gas status type ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum GasStatus {
    Safe,
    Warning,
    Danger,
}

impl GasStatus {
    fn from_raw(raw: u16) -> Self {
        if raw < SAFE_THRESHOLD {
            GasStatus::Safe
        } else if raw < WARNING_THRESHOLD {
            GasStatus::Warning
        } else {
            GasStatus::Danger
        }
    }

    fn label(&self) -> &'static str {
        match self {
            GasStatus::Safe => "SAFE",
            GasStatus::Warning => "WARNING",
            GasStatus::Danger => "DANGER",
        }
    }
}

// ── Moving-average filter ───────────────────────────────────────────────────

struct MovingAverage {
    buffer: [u16; FILTER_WINDOW],
    index: usize,
    filled: bool,
}

impl MovingAverage {
    const fn new() -> Self {
        Self {
            buffer: [0u16; FILTER_WINDOW],
            index: 0,
            filled: false,
        }
    }

    fn update(&mut self, sample: u16) -> u16 {
        self.buffer[self.index] = sample;
        self.index = (self.index + 1) % FILTER_WINDOW;
        if self.index == 0 {
            self.filled = true;
        }

        let len = if self.filled { FILTER_WINDOW } else { self.index.max(1) };
        let sum: u32 = self.buffer[..len].iter().map(|&v| v as u32).sum();
        (sum / len as u32) as u16
    }
}

// ── OLED drawing helpers ────────────────────────────────────────────────────

type OledDisplay<'a> = Ssd1306<
    I2CInterface<I2cDriver<'a>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

fn draw_screen(display: &mut OledDisplay<'_>, ppm: f32, raw: u16, status: GasStatus) {
    display.clear(BinaryColor::Off).unwrap();

    let title_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X18_BOLD)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Methane Detector", Point::new(0, 0), title_style, Baseline::Top)
        .draw(display)
        .unwrap();

    Rectangle::new(Point::new(0, 18), Size::new(128, 1))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::On)
                .build(),
        )
        .draw(display)
        .unwrap();

    let small_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut ppm_str = heapless::String::<32>::new();
    core::fmt::write(&mut ppm_str, format_args!("Gas: {:.0} ppm", ppm)).unwrap();
    Text::with_baseline(ppm_str.as_str(), Point::new(0, 22), small_style, Baseline::Top)
        .draw(display)
        .unwrap();

    let mut raw_str = heapless::String::<32>::new();
    core::fmt::write(&mut raw_str, format_args!("Raw ADC: {}", raw)).unwrap();
    Text::with_baseline(raw_str.as_str(), Point::new(0, 34), small_style, Baseline::Top)
        .draw(display)
        .unwrap();

    let mut status_str = heapless::String::<16>::new();
    core::fmt::write(&mut status_str, format_args!("Status: {}", status.label())).unwrap();

    let (text_color, bg_color) = match status {
        GasStatus::Safe => (BinaryColor::On, BinaryColor::Off),
        GasStatus::Warning => (BinaryColor::Off, BinaryColor::On), 
        GasStatus::Danger => (BinaryColor::Off, BinaryColor::On),  
    };

    if bg_color == BinaryColor::On {
        Rectangle::new(Point::new(0, 46), Size::new(128, 12))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(BinaryColor::On)
                    .build(),
            )
            .draw(display)
            .unwrap();
    }

    let status_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(text_color)
        .build();

    Text::with_baseline(
        status_str.as_str(),
        Point::new(2, 46),
        status_style,
        Baseline::Top,
    )
    .draw(display)
    .unwrap();

    let bar_width = ((raw as u32 * 128) / 4095).min(128) as u32;
    if bar_width > 0 {
        Rectangle::new(Point::new(0, 60), Size::new(bar_width, 4))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(BinaryColor::On)
                    .build(),
            )
            .draw(display)
            .unwrap();
    }
    
    Rectangle::new(Point::new(0, 60), Size::new(128, 4))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(BinaryColor::On)
                .stroke_width(1)
                .build(),
        )
        .draw(display)
        .unwrap();
}

// ── Entry point ─────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("=== Methane Gas Detector Starting ===");

    let peripherals = Peripherals::take().unwrap();

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let i2c_config = I2cConfig::new().baudrate(400_u32.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &i2c_config)?;

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    display.flush().unwrap();
    info!("OLED display initialised");

    // ── FIXED ESP-IDF-HAL 0.46 ADC INITIALIZATION ────────────────────────────
    
    // 1. The AdcDriver ONLY takes the peripheral.
    let adc_driver = AdcDriver::new(peripherals.adc1)?;
    
    // 2. AdcChannelConfig is a struct, not a builder. 
    // We instantiate it directly and use `..Default::default()` for the rest (like resolution).
    let config = AdcChannelConfig {
        attenuation: adc::attenuation::DB_12,
        // Removed `calibration: true` – we let Default handle the proper type!
        ..Default::default()
    };

    // 3. AdcChannelDriver takes 3 arguments: &driver, pin, &config.
    let mut adc_pin = AdcChannelDriver::new(
        &adc_driver,
        peripherals.pins.gpio34,
        &config,
    )?;
    
    info!("ADC initialised on GPIO34");

    let mut buzzer = PinDriver::output(peripherals.pins.gpio25)?;
    buzzer.set_low()?;

    let mut led = PinDriver::output(peripherals.pins.gpio26)?;
    led.set_low()?;
    info!("GPIO outputs (buzzer=25, led=26) initialised");

    let mut filter = MovingAverage::new();
    let mut peak_raw: u16 = 0;
    let mut loop_count: u32 = 0;

    info!("Entering main loop…");

    loop {
        loop_count = loop_count.wrapping_add(1);

        let raw_sample = adc_pin.read().unwrap_or(0);
        
        let raw = filter.update(raw_sample);
        let ppm = raw as f32 * PPM_SCALE;
        let status = GasStatus::from_raw(raw);

        if raw > peak_raw {
            peak_raw = raw;
        }

        match status {
            GasStatus::Safe => {
                buzzer.set_low()?;
                led.set_low()?;
            }
            GasStatus::Warning => {
                if loop_count % 2 == 0 {
                    buzzer.set_high()?;
                    led.set_high()?;
                } else {
                    buzzer.set_low()?;
                    led.set_low()?;
                }
            }
            GasStatus::Danger => {
                buzzer.set_high()?;
                led.set_high()?;
            }
        }

        draw_screen(&mut display, ppm, raw, status);
        display.flush().unwrap();

        info!(
            "[Loop {}] Raw={} Filtered={} PPM={:.0} Status={:?} Peak={}",
            loop_count, raw_sample, raw, ppm, status, peak_raw
        );

        FreeRtos::delay_ms(LOOP_DELAY_MS);
    }
}
