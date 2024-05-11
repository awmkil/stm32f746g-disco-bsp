#![no_main]
#![no_std]

use core::ops::Range;

use panic_semihosting as _;
use stm32f7xx_hal as _;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use stm32f7xx_hal::{
    dma::DMA,
    gpio::Speed,
    pac,
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

use wm8994::{registers::FAMILY_ID, Wm8994};

const HSE_CLOCK_HZ: u32 = 25_000_000;
const SYS_CLOCK_HZ: u32 = 216_000_000;

const AUDIO_OUT_FREQ: u32 = 16_000;
const VALID_ADDR_RANGE: Range<u8> = 0x08..0x78;

//38 1a

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    hprintln!("Program start...");

    let mut rcc = dp.RCC.constrain();
    let clocks = rcc
        .cfgr
        .hse(HSEClock::new(HSE_CLOCK_HZ.Hz(), HSEClockMode::Bypass))
        .sysclk(SYS_CLOCK_HZ.Hz())
        .hclk(SYS_CLOCK_HZ.Hz())
        .freeze();

    // let gpioi = dp.GPIOI.split();
    // let gpiog = dp.GPIOG.split();
    let gpioh = dp.GPIOH.split();

    // let sai_pins = (
    //     gpioi.pi4.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_MCK_A
    //     gpioi.pi5.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SCK_A
    //     gpioi.pi6.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SD_A
    //     gpioi.pi7.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_FS_A
    //     gpiog.pg10.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SD_B
    // );

    let i2c_pins = (
        gpioh.ph7.into_alternate_open_drain::<4>(), // I2C3_SCL
        gpioh.ph8.into_alternate_open_drain::<4>(), // I2C3_SDA
    );

    let mut i2c = stm32f7xx_hal::i2c::BlockingI2c::i2c3(
        dp.I2C3,
        i2c_pins,
        stm32f7xx_hal::i2c::Mode::fast(100.kHz()),
        &clocks,
        &mut rcc.apb1,
        50_000,
    );

    // let dma = DMA::new(dp.DMA2);
    // let mut audio_out_stream = dma.streams.stream4; // Channel 3
    // let mut audio_in_stream = dma.streams.stream7; // Channel 0
    // let dma = dma.handle.enable(&mut rcc.ahb1);

    let delay = cp.SYST.delay(&clocks);

    let mut driver = Wm8994::new(wm8994::Config { address: 0x1a }, i2c, delay);

    if let Ok(FAMILY_ID) = driver.get_family() {
        hprintln!("Detected DAC with id {:?}", FAMILY_ID);
    }

    loop {

    }
}
