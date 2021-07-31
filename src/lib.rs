use wasm_bindgen::prelude::*;
use hound::{WavSpec, SampleFormat};
use rustfft::{FftPlanner, num_complex::Complex};
use std::io::Cursor;

const AUDACITY_MAX_FREQ: u32 = 8000;

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
    let mut luma = r as f32 * 0.2126;
    luma += g as f32 * 0.7152;
    luma += b as f32 * 0.0722;
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

    fn to_complex_rows(&self) -> Vec<Vec<Complex<f32>>> {
        let mut rows = Vec::new();
        for i in 0..self.height {
            let mut row = Vec::new();
            for j in 0..self.width {
                let idx = i * self.width + j;
                row.push((self.data[idx] as f32).into());
            }
            rows.push(row);
        }
        rows
    }
}

#[wasm_bindgen]
pub struct Audio {
    samples: Vec<f32>,
    pub sample_rate: usize,
}

#[wasm_bindgen]
impl Audio {
    pub fn samples(&self) -> Vec<f32> {
        self.samples.clone()
    }

    fn normalize(&mut self) {
        let max = *self.samples.iter()
            .max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap())
            .unwrap();
        self.samples.iter_mut().for_each(|s| *s = *s/max);
    }

    pub fn to_wav(&self, level: f32) -> Vec<u8> {
        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut buf = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut buf, spec).unwrap();
        for s in &self.samples {
            writer.write_sample((level * s * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();
        buf.into_inner()
    }
}

#[wasm_bindgen]
pub fn image_to_audio(data: Vec<u8>, width: usize, height: usize, sample_rate: usize) -> Audio {
    let mut img = Image::new(data, width, height);

    // zero-pad the top of the image so it "fits" into Audacity's 8kHz default spectrogram view
    let scaling_factor = sample_rate as f32 / (2.0 * AUDACITY_MAX_FREQ as f32);
    let padded_height = (height as f32 * scaling_factor) as usize;
    img.pad_top(padded_height - height);

    // invert so that darker pixels result in high values
    img.invert();

    // rotate 90 degrees clockwise, since our IFFT wants to operate on image cols 
    img.rotate90();

    // add a symmetric set of "frequencies" to the end to create something that results in a real-
    // valued waveform
    img.reflect_about_y_axis();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(2 * padded_height);
    let mut audio = Audio {
        samples: Vec::new(),
        sample_rate,
    };

    // for each row of data (i.e. column of the image), perform an FFT, and concat the resulting
    // waveform to our result
    for mut row in img.to_complex_rows() {
        fft.process(&mut row);
        // we only need the real components (complex should be zero)
        audio.samples.extend(row.iter().map(|c| c.re));
    }

    // remove the DC bias
    let avg = audio.samples.iter().sum::<f32>() / audio.samples.len() as f32;
    audio.samples.iter_mut().for_each(|s| *s = *s - avg);

    // normalize to [-1, 1]
    audio.normalize();

    audio
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
