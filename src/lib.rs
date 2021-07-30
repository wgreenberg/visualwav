use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Image {
    data: Vec<u8>,
    pub height: usize,
    pub width: usize,
}

fn rgb_to_luma(r: u8, g: u8, b: u8, a: u8) -> u8 {
    if a < 10 {
        return 255;
    }
    let mut luma = r as f64 * 0.2126;
    luma += g as f64 * 0.7152;
    luma += b as f64 * 0.0722;
    luma as u8
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
impl Image {
    pub fn new(data: Vec<u8>, width: usize, height: usize) -> Image {
        let mut grayscale = Vec::new();
        for px in data.chunks(4) {
            grayscale.push(rgb_to_luma(px[0], px[1], px[2], px[3]));
        }
        Image { data: grayscale, height, width }
    }

    pub fn pad_top(&mut self, n_rows: usize) {
        let mut padding = vec![255; n_rows * self.width];
        self.height += n_rows;
        padding.extend(&self.data);
        self.data = padding;
    }

    pub fn invert(&mut self) {
        for luma in self.data.iter_mut() {
            *luma = 255 - *luma;
        }
    }

    pub fn rgba_data(&self) -> Vec<u8> {
        let mut rgb_data = Vec::new();
        for &luma in &self.data {
            rgb_data.push(luma);
            rgb_data.push(luma);
            rgb_data.push(luma);
            rgb_data.push(255);
        }
        rgb_data
    }

    pub fn rotate90(&mut self) {
        let mut rotated = Vec::with_capacity(self.width * self.height);
        for i in 0..self.width {
            for j in (0..self.height).rev() {
                let idx = self.width * j + i;
                rotated.push(self.data[idx]);
            }
        }
        let (width, height) = (self.width, self.height);
        self.width = height;
        self.height = width;
        self.data = rotated;
    }

    pub fn reflect_about_y_axis(&mut self) {
        let mut reflected = Vec::with_capacity(2 * self.width * self.height);
        for i in 0..self.height {
            let start = i * self.width;
            let end = (i+1) * self.width;
            let slice = &self.data[start..end];
            reflected.extend_from_slice(&slice);
            reflected.extend(slice.iter().rev());
        }
        self.width *= 2;
        self.data = reflected;
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
