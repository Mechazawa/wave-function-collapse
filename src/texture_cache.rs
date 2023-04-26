use image::DynamicImage;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub struct TextureCache<'a> {
    texture_creator: &'a TextureCreator<WindowContext>,
    cache: HashMap<u64, Texture>,
}

impl<'a> TextureCache<'a> {
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Self {
        Self {
            texture_creator,
            cache: HashMap::new(),
        }
    }

    pub fn get_or_insert(&mut self, image: &DynamicImage) -> Result<&Texture, String> {
        let hash = self.hash_image(image);

        if !self.cache.contains_key(&hash) {
            let texture = self.image_to_texture(image)?;
            self.cache.insert(hash, texture);
        }

        Ok(self.cache.get(&hash).unwrap())
    }

    fn hash_image(&self, image: &DynamicImage) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        image.as_bytes().hash(&mut hasher);
        hasher.finish()
    }

    fn image_to_texture(&self, image: &DynamicImage) -> Result<Texture, String> {
        let (width, height) = image.dimensions();
        let pixel_format = sdl2::pixels::PixelFormatEnum::RGBA32;
        let mut texture = self
            .texture_creator
            .create_texture_streaming(pixel_format, width, height)
            .map_err(|e| e.to_string())?;

        let pitch = width * 4;
        let image_rgba = image.to_rgba8();
        texture
            .with_lock(None, |buffer: &mut [u8], _: usize| {
                buffer.copy_from_slice(&image_rgba);
            })
            .map_err(|e| e.to_string())?;

        Ok(texture)
    }
}
