use image::{DynamicImage, GenericImageView, Pixel};
use std::hash::{Hash, Hasher};
use num_traits::cast::ToPrimitive;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{ImageData, CanvasRenderingContext2d};

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

#[cfg(target_arch = "wasm32")]
impl Sprite {
    pub fn to_image_data(&self) -> Result<ImageData, JsValue> {
        let rgba = self.image.to_rgba8();
        let (width, height) = self.image.dimensions();
        
        ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(rgba.as_raw()),
            width,
            height
        )
    }
    
    pub fn from_image_data(image_data: &ImageData) -> Result<Self, JsValue> {
        let width = image_data.width();
        let height = image_data.height();
        let data = image_data.data();
        
        let rgba_data: Vec<u8> = data.0.iter().cloned().collect();
        
        let image = image::RgbaImage::from_raw(width, height, rgba_data)
            .ok_or("Failed to create image from ImageData")?;
        
        Ok(Sprite {
            image: DynamicImage::ImageRgba8(image),
        })
    }
    
    pub fn draw_to_canvas(&self, ctx: &CanvasRenderingContext2d, x: f64, y: f64) -> Result<(), JsValue> {
        let image_data = self.to_image_data()?;
        ctx.put_image_data(&image_data, x, y)?;
        Ok(())
    }
}