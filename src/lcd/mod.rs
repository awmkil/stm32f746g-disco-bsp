use embedded_graphics::draw_target::DrawTarget;

mod rk043fn48h;
// mod simulator;

pub type Stm32F746DiscoLcd = rk043fn48h::Rk043fn48h;

pub trait Lcd: DrawTarget {
    type Config;

    fn get_config() -> Self::Config;
}
