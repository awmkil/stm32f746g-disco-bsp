use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions, Size},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{self, PointsIter},
    Pixel,
};

use stm32f7xx_hal::{
    ltdc::{DisplayConfig, DisplayController, Layer, PixelFormat},
    pac::{DMA2D, LTDC},
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

use crate::lcd::Lcd;

const RK043FN48H_WIDTH: u16 = 480;
const RK043FN48H_HEIGHT: u16 = 272;

// Graphics framebuffer
const FB_SIZE: usize = (RK043FN48H_WIDTH as usize) * (RK043FN48H_HEIGHT as usize);
static mut FB_LAYER1: [u16; FB_SIZE] = [0; FB_SIZE];

pub struct Rk043fn48h {
    pub config: DisplayConfig,
    pub controller: DisplayController<u16>,
}

impl Rk043fn48h {
    pub fn new(ltdc: LTDC, dma2d: DMA2D) -> Self {
        let config = Rk043fn48h::get_config();
        let mut controller = DisplayController::new(
            ltdc,
            dma2d,
            PixelFormat::RGB565,
            Rk043fn48h::get_config(),
            Some(&HSEClock::new(25_000_000.Hz(), HSEClockMode::Bypass)),
        );

        controller.config_layer(
            Layer::L1,
            unsafe { &mut *core::ptr::addr_of_mut!(FB_LAYER1) },
            PixelFormat::RGB565,
        );
        controller.enable_layer(Layer::L1);
        controller.reload();

        Self { config, controller }
    }
}

impl Lcd for Rk043fn48h {
    type Config = DisplayConfig;

    fn get_config() -> Self::Config {
        DisplayConfig {
            active_width: RK043FN48H_WIDTH,
            active_height: RK043FN48H_HEIGHT,
            h_back_porch: 13,
            h_front_porch: 30,
            h_sync: 41,
            v_back_porch: 2,
            v_front_porch: 2,
            v_sync: 10,
            frame_rate: 60,
            h_sync_pol: false,
            v_sync_pol: false,
            no_data_enable_pol: false,
            pixel_clock_pol: false,
        }
    }
}

impl DrawTarget for Rk043fn48h {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::prelude::Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            let value: u16 = (color.b() as u16 & 0x1F)
                | ((color.g() as u16 & 0x3F) << 5)
                | ((color.r() as u16 & 0x1F) << 11);
            self.controller
                .draw_pixel(Layer::L1, coord.x as usize, coord.y as usize, value);
        }

        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        // Clamp area to drawable part of the display target
        let drawable_area = area.intersection(&self.bounding_box());
        let points = area.points();

        // Check that there are visible pixels to be drawn
        if drawable_area.size != Size::zero() {
            self.draw_iter(
                points
                    .zip(colors)
                    .filter(|(pos, _color)| drawable_area.contains(*pos))
                    .map(|(pos, color)| Pixel(pos, color)),
            )
        } else {
            Ok(())
        }
    }

    fn fill_solid(
        &mut self,
        area: &primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        self.fill_contiguous(area, core::iter::repeat(color))
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_solid(&self.bounding_box(), color)
    }
}

impl OriginDimensions for Rk043fn48h {
    fn size(&self) -> Size {
        Size::new(
            self.config.active_width as u32,
            self.config.active_height as u32,
        )
    }
}
