#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Angle, Point},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Circle, Primitive, PrimitiveStyleBuilder, Sector},
    Drawable,
};
use panic_semihosting as _;
use stm32f746g_disco_bsp::lcd::{Lcd, Stm32F746DiscoLcd};
use stm32f7xx_hal::{
    gpio::Speed,
    pac,
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

const STEPS: i32 = 10;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let gpioe = dp.GPIOE.split();
    let gpiog = dp.GPIOG.split();
    let gpioh = dp.GPIOH.split();
    let gpioi = dp.GPIOI.split();
    let gpioj = dp.GPIOJ.split();
    let gpiok = dp.GPIOK.split();

    let _lcd_pins = (
        gpioe.pe4.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_B0
        gpiog.pg12.into_alternate::<9>().set_speed(Speed::VeryHigh), // LTCD_B4
        gpioi.pi9.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_VSYNC
        gpioi.pi10.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_HSYNC
        gpioi.pi13.into_alternate::<14>().set_speed(Speed::VeryHigh),
        gpioi.pi14.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_CLK
        gpioi.pi15.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_R0
        gpioj.pj0.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R1
        gpioj.pj1.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R2
        gpioj.pj2.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R3
        gpioj.pj3.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R4
        gpioj.pj4.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R5
        gpioj.pj5.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R6
        gpioj.pj6.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_R7
        gpioj.pj7.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G0
        gpioj.pj8.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G1
        gpioj.pj9.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G2
        gpioj.pj10.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_G3
        gpioj.pj11.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_G4
        gpioj.pj13.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_B1
        gpioj.pj14.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_B2
        gpioj.pj15.into_alternate::<14>().set_speed(Speed::VeryHigh), // LTCD_B3
        gpiok.pk0.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G5
        gpiok.pk1.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G6
        gpiok.pk2.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_G7
        gpiok.pk4.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_B5
        gpiok.pk5.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_B6
        gpiok.pk6.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_D7
        gpiok.pk7.into_alternate::<14>().set_speed(Speed::VeryHigh),  // LTCD_E
    );

    let _hse_pin = gpioh.ph1.into_floating_input(); // HSE osc out in High Z

    let mut lcd_on_pin = gpioi.pi12.into_push_pull_output();
    lcd_on_pin.set_low();

    let mut lcd_backlight_pin = gpiok.pk3.into_push_pull_output();
    lcd_backlight_pin.set_low();

    let ltdc = dp.LTDC;
    let dma2d = dp.DMA2D;

    // Setup delay
    let rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .hse(HSEClock::new(25_000_000.Hz(), HSEClockMode::Bypass))
        .sysclk(216_000_000.Hz())
        .hclk(216_000_000.Hz())
        .freeze();
    let mut delay = cp.SYST.delay(&clocks);

    lcd_on_pin.set_high();
    lcd_backlight_pin.set_high();
    let mut display = Stm32F746DiscoLcd::new(ltdc, dma2d);
    let _ = display.clear(Rgb565::WHITE);

    let pacman_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLACK)
        .stroke_width(2)
        .fill_color(Rgb565::YELLOW)
        .build();
    let eye_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLACK)
        .stroke_width(1)
        .fill_color(Rgb565::BLACK)
        .build();
    let bg_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::WHITE)
        .stroke_width(2)
        .stroke_color(Rgb565::WHITE)
        .build();

    let pacman_position = Point::new(
        ((Stm32F746DiscoLcd::get_config().active_width / 2)
            - (Stm32F746DiscoLcd::get_config().active_height / 2)) as i32,
        5,
    );
    let eye_position = Point::new(
        (Stm32F746DiscoLcd::get_config().active_width / 2 + 32) as i32,
        (Stm32F746DiscoLcd::get_config().active_height / 2 - 80) as i32,
    );
    let mut progress: i32 = 0;
    loop {
        let p = (progress - STEPS).abs();

        // Fill negative space with white
        let _ = Sector::new(
            pacman_position,
            (Stm32F746DiscoLcd::get_config().active_height - 10) as u32,
            Angle::from_degrees((360 - (p * 30 / STEPS)) as f32),
            Angle::from_degrees((360 - (360 - 2 * p * 30 / STEPS)) as f32),
        )
        .into_styled(bg_style)
        .draw(&mut display);

        // Draw a Sector as the main Pacman feature.
        let _ = Sector::new(
            pacman_position,
            (Stm32F746DiscoLcd::get_config().active_height - 10) as u32,
            Angle::from_degrees((p * 30 / STEPS) as f32),
            Angle::from_degrees((360 - 2 * p * 30 / STEPS) as f32),
        )
        .into_styled(pacman_style)
        .draw(&mut display);

        // Draw a Circle as the eye.
        let _ = Circle::new(eye_position, 15)
            .into_styled(eye_style)
            .draw(&mut display);

        delay.delay_ms(10_u32);

        progress = (progress + 1) % (2 * STEPS + 1);
    }
}
