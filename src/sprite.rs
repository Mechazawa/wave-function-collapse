use ggez::{graphics::{Image, ImageFormat}, Context};
use image::{DynamicImage, GenericImageView, Pixel};
use std::hash::{Hash, Hasher};
use num_traits::cast::ToPrimitive;

#[derive(Debug, Clone)]
pub struct Sprite {
    /// Todo either figure out other purposes or phase out struct
    pub image: DynamicImage,
}

impl Hash for Sprite {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pixel in self.image.pixels() {
            for channel in pixel.2.channels() {
                if let Some(value) = channel.to_u8() {
                    state.write_u8(value)
                }
            }
        }
    }
}

impl Sprite {
    pub fn into_image(self, ctx: &mut Context) -> Image {
        let (width, height) = self.image.dimensions();
        let rgba = self.image.to_rgba8();
        let rgba_data = rgba.as_raw();

        Image::from_pixels(ctx, &rgba_data, ImageFormat::Rgba8Sint, width, height)
    }
}