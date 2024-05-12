#![no_main]
#![no_std]

use panic_semihosting as _;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use stm32f7xx_hal::{
    dma::DMA, gpio::Speed, interrupt, pac, prelude::*, rcc::{HSEClock, HSEClockMode}
};

use wm8994::{registers::FAMILY_ID, Wm8994};

const HSE_CLOCK_HZ: u32 = 25_000_000;

pub const BLOCK_LENGTH: usize = 32;
pub const HALF_DMA_BUFFER_LENGTH: usize = BLOCK_LENGTH * 2;
pub const DMA_BUFFER_LENGTH: usize = HALF_DMA_BUFFER_LENGTH * 2;
#[link_section = ".sram1_bss"]
static mut TX_BUFFER: [i16; DMA_BUFFER_LENGTH] = [0; DMA_BUFFER_LENGTH];

// 375 Hz @48kHz.
const SINE_375_HZ: [i16; DMA_BUFFER_LENGTH] = [
    0, 1607, 3211, 4807, 6392, 7961, 9511, 11038, 12539, 14009, 15446, 16845, 18204, 19519, 20787,
    22004, 23169, 24278, 25329, 26318, 27244, 28105, 28897, 29621, 30272, 30851, 31356, 31785,
    32137, 32412, 32609, 32727, 32767, 32727, 32609, 32412, 32137, 31785, 31356, 30851, 30272,
    29621, 28897, 28105, 27244, 26318, 25329, 24278, 23169, 22004, 20787, 19519, 18204, 16845,
    15446, 14009, 12539, 11038, 9511, 7961, 6392, 4807, 3211, 1607, 0, -1607, -3211, -4807, -6392,
    -7961, -9511, -11038, -12539, -14009, -15446, -16845, -18204, -19519, -20787, -22004, -23169,
    -24278, -25329, -26318, -27244, -28105, -28897, -29621, -30272, -30851, -31356, -31785, -32137,
    -32412, -32609, -32727, -32767, -32727, -32609, -32412, -32137, -31785, -31356, -30851, -30272,
    -29621, -28897, -28105, -27244, -26318, -25329, -24278, -23169, -22004, -20787, -19519, -18204,
    -16845, -15446, -14009, -12539, -11038, -9511, -7961, -6392, -4807, -3211, -1607,
];

#[entry]
fn main() -> ! {
    let sample_rate = 48_000;

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Enable SAI2 clock
    dp.RCC.apb2enr.write(|w| w.sai2en().enabled());

    // Configure SAI2 Clock source to be PLL I2S
    dp.RCC.dckcfgr1.write(|w| w.sai2sel().plli2s());

    // Enable DMA2 clock
    dp.RCC.ahb1enr.write(|w| w.dma2en().enabled());

    let mut rcc = dp.RCC.constrain();

    let clocks = if sample_rate == 8_000
        || sample_rate == 16_000
        || sample_rate == 48_000
        || sample_rate == 96_000
    {
        rcc.cfgr
            .set_defaults()
            .hse(HSEClock::new(25_000_000.Hz(), HSEClockMode::Bypass))
            .plli2sn(429)
            .plli2sq(2)
            .use_plli2s()
            .freeze()
    } else if sample_rate == 11_025 || sample_rate == 22_050 || sample_rate == 44_100 {
        rcc.cfgr
            .set_defaults()
            .hse(HSEClock::new(HSE_CLOCK_HZ.Hz(), HSEClockMode::Bypass))
            .plli2sn(344)
            .plli2sq(7)
            .use_plli2s()
            .freeze()
    } else {
        panic!("Invalid sample rate");
    };

    let gpioi = dp.GPIOI.split();
    let gpiog = dp.GPIOG.split();
    let gpioh = dp.GPIOH.split();

    let _sai_pins = (
        gpioi.pi4.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_MCK_A
        gpioi.pi5.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SCK_A
        gpioi.pi6.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SD_A
        gpioi.pi7.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_FS_A
        gpiog.pg10.into_alternate::<10>().set_speed(Speed::VeryHigh), // SAI2_SD_B
    );

    // Finish using https://github.com/blipp/stm32f7-discovery/blob/master/src/audio.rs#L97
    // https://github.com/STMicroelectronics/32f746gdiscovery-bsp/blob/main/stm32746g_discovery_audio.c#L699

    let sai2 = dp.SAI2;

    sai2.cha.clrfr.write(|w| w.clfsdet().clear()); // Clear late frame synchronization detection flag
    sai2.cha.clrfr.write(|w| w.cafsdet().clear()); // Clear anticipated frame synchronization detection flag
    sai2.cha.clrfr.write(|w| w.ccnrdy().clear()); // Clear codec not ready flag
    sai2.cha.clrfr.write(|w| w.cwckcfg().clear()); // Clear wrong clock configuration flag
    sai2.cha.clrfr.write(|w| w.cmutedet().clear()); // Clear mute detection flag
    sai2.cha.clrfr.write(|w| w.covrudr().clear()); // Clear overrun / underrun
    sai2.cha.cr2.write(|w| w.fflush().flush()); // Flush the fifo
    sai2.gcr.write(|w| unsafe { w.syncout().bits(0) }); // Disable synchronization outputs

    // Channel A (Output) Configuration
    sai2.cha.cr1.write(|w| w.saien().disabled());

    // Channel A (Output) ACR1&2 Configuration
    sai2.cha.cr1.write(|w| w.mode().master_tx()); // SAI_MODEMASTER_TX
    sai2.cha.cr1.write(|w| w.nodiv().no_div()); // SAI_MASTERDIVIDER_ENABLED - Not sure -
                                                // sai2.cha.cr1.write(|w| unsafe { w.mckdiv().bits(????) }); // SAI_MASTERDIVIDER_ENABLED - Not sure -
    sai2.cha.cr1.write(|w| w.prtcfg().free()); // SAI_FREE_PROTOCOL
    sai2.cha.cr1.write(|w| w.ds().bit16()); // SAI_DATASIZE_16
    sai2.cha.cr1.write(|w| w.lsbfirst().msb_first()); // SAI_FIRSTBIT_MSB
    sai2.cha.cr1.write(|w| w.ckstr().rising_edge()); // SAI_CLOCKSTROBING_RISINGEDGE
    sai2.cha.cr1.write(|w| w.syncen().asynchronous()); // SAI_ASYNCHRONOUS
    sai2.cha.cr1.write(|w| w.outdriv().immediately()); // SAI_OUTPUTDRIVE_ENABLED - Not sure -
    sai2.cha.cr2.write(|w| w.fth().quarter1()); // SAI_FIFOTHRESHOLD_1QF

    // Channel A (Output) Frame Configuration
    sai2.cha.frcr.write(|w| unsafe { w.frl().bits(64 - 1) }); // Frame Length: 64
    sai2.cha.frcr.write(|w| unsafe { w.fsall().bits(32 - 1) }); // Frame active Length: 32
    sai2.cha.frcr.write(|w| w.fsdef().set_bit()); // SAI_FS_CHANNEL_IDENTIFICATION (FS Definition: Start frame + Channel Side identification) - Not sure -
    sai2.cha.frcr.write(|w| w.fspol().falling_edge()); // SAI_FS_ACTIVE_LOW
    sai2.cha.frcr.write(|w| w.fsoff().before_first()); // SAI_FS_BEFOREFIRSTBIT

    // Channel A (Output) Slot Configuration
    sai2.cha.slotr.write(|w| unsafe { w.fboff().bits(0) }); // Slot First Bit Offset: 0
    sai2.cha.slotr.write(|w| w.slotsz().bit16()); //   Slot Size  : 16
    sai2.cha.slotr.write(|w| unsafe { w.nbslot().bits(4 - 1) }); // Slot Number: 4
    sai2.cha.slotr.write(|w| w.sloten().active()); // Slot Active: All slot actives

    sai2.cha.cr1.write(|w| w.saien().enabled());

    // DMA2 - Stream 4 (Output) Configuration
    dp.DMA2.st[4].cr.write(|w| w.en().disabled());

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::DMA2_STREAM4);
    }

    dp.DMA2.st[4].cr.write(|w| w.chsel().bits(3)); // Channel 3
    dp.DMA2.st[4].cr.write(|w| w.dir().memory_to_peripheral()); // DMA_MEMORY_TO_PERIPH
    dp.DMA2.st[4].cr.write(|w| w.pinc().clear_bit()); // DMA_PINC_DISABLE
    dp.DMA2.st[4].cr.write(|w| w.minc().set_bit()); // DMA_MINC_ENABLE
    dp.DMA2.st[4].cr.write(|w| w.circ().set_bit()); // DMA_CIRCULAR
    dp.DMA2.st[4].cr.write(|w| w.pl().high()); // DMA_PRIORITY_HIGH
    dp.DMA2.st[4].cr.write(|w| w.mburst().single()); // DMA_MBURST_SINGLE
    dp.DMA2.st[4].cr.write(|w| w.pburst().single()); // DMA_PBURST_SINGLE
    dp.DMA2.st[4].fcr.write(|w| w.dmdis().disabled()); // DMA_FIFOMODE_ENABLE (disable direct mode)
    dp.DMA2.st[4].fcr.write(|w| w.fth().full()); // DMA_FIFO_THRESHOLD_FULL

    // Set DMA peripheral address to SAI2 CHA & configure buffers
    let sai2par = &sai2.cha.dr as *const _ as u32;
    dp.DMA2.st[4].par.write(|w| unsafe { w.pa().bits(sai2par) });
    let tx_buffer_ptr = unsafe { TX_BUFFER.as_ptr() as usize as u32 };
    dp.DMA2.st[4]
        .m0ar
        .write(|w| unsafe { w.m0a().bits(tx_buffer_ptr) });
    let tx_buffer_length = DMA_BUFFER_LENGTH as u16;
    dp.DMA2.st[4].ndtr.write(|w| w.ndt().bits(tx_buffer_length));

    dp.DMA2.st[4].cr.write(|w| w.en().enabled());

    sai2.cha.cr1.write(|w| w.dmaen().enabled());

    let i2c_pins = (
        gpioh.ph7.into_alternate_open_drain::<4>(), // I2C3_SCL
        gpioh.ph8.into_alternate_open_drain::<4>(), // I2C3_SDA
    );

    let i2c = stm32f7xx_hal::i2c::BlockingI2c::i2c3(
        dp.I2C3,
        i2c_pins,
        stm32f7xx_hal::i2c::Mode::fast(100.kHz()),
        &clocks,
        &mut rcc.apb1,
        50_000,
    );

    let delay = cp.SYST.delay(&clocks);

    let mut driver = Wm8994::new(wm8994::Config { address: 0x1a }, i2c, delay);

    if let Ok(FAMILY_ID) = driver.get_family() {
        hprintln!("WM8994 dectected on I2C3");
    }

    if let Ok(()) = driver.init_headphone() {
        hprintln!("WM8994 init OK");
    }

    loop {
        unsafe {
            TX_BUFFER.clone_from_slice(&SINE_375_HZ);
        }
    }
}


#[interrupt]
fn DMA2_STREAM4() {
    hprintln!("DMA2_STREAM4");
}