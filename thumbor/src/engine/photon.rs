use std::io::Cursor;

use bytes::Bytes;

use photon_rs::{
    effects, filters, multiple, native::open_image_from_bytes, transform, PhotonImage,
};

use super::{Engine, SpecTransform};
use crate::pb::*;
use image::{DynamicImage, ImageBuffer, ImageFormat};
use lazy_static::lazy_static;

lazy_static! {
    static ref WATERMARK: PhotonImage = {
        let data = include_bytes!("../../rust-logo.png");
        let watermark = open_image_from_bytes(data).unwrap();
        // transform::resize(&watermark, 64, 64, resize::SampleFilter::CatmullRom)
        transform::resize(&watermark, 64, 64, transform::SamplingFilter::Nearest)
    };
}

pub struct Photon(PhotonImage);

impl TryFrom<Bytes> for Photon {
    type Error = anyhow::Error;

    fn try_from(data: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(open_image_from_bytes(&data)?))
    }
}

impl Engine for Photon {
    fn apply(&mut self, specs: &[Spec]) {
        for spec in specs.iter() {
            match spec.data {
                Some(spec::Data::Crop(ref v)) => self.transform(v),
                Some(spec::Data::Contrast(ref v)) => self.transform(v),
                Some(spec::Data::Filter(ref v)) => self.transform(v),
                Some(spec::Data::Fliph(ref v)) => self.transform(v),
                Some(spec::Data::Flipv(ref v)) => self.transform(v),
                Some(spec::Data::Resize(ref v)) => self.transform(v),
                Some(spec::Data::Watermark(ref v)) => self.transform(v),
                _ => {}
            }
        }
    }

    fn generate(self, format: ImageFormat) -> Vec<u8> {
        image_to_buf(self.0, format)
    }
}

impl SpecTransform<&Crop> for Photon {
    fn transform(&mut self, op: &Crop) {
        let img = transform::crop(&mut self.0, op.x1, op.y1, op.x2, op.y2);
        self.0 = img;
    }
}

impl SpecTransform<&Contrast> for Photon {
    fn transform(&mut self, op: &Contrast) {
        effects::adjust_contrast(&mut self.0, op.contrast)
    }
}

impl SpecTransform<&Flipv> for Photon {
    fn transform(&mut self, _op: &Flipv) {
        transform::flipv(&mut self.0)
    }
}

impl SpecTransform<&Fliph> for Photon {
    fn transform(&mut self, _op: &Fliph) {
        transform::fliph(&mut self.0)
    }
}

impl SpecTransform<&Filter> for Photon {
    fn transform(&mut self, op: &Filter) {
        match filter::Filter::from_i32(op.filter) {
            Some(filter::Filter::Unspecified) => {}
            Some(f) => filters::filter(&mut self.0, f.to_str().unwrap()),
            _ => {}
        }
    }
}

impl SpecTransform<&Resize> for Photon {
    fn transform(&mut self, op: &Resize) {
        let img = match resize::ResizeType::from_i32(op.rtype).unwrap() {
            resize::ResizeType::Normal => transform::resize(
                &mut self.0,
                op.width,
                op.height,
                resize::SampleFilter::from_i32(op.filter).unwrap().into(),
            ),
            resize::ResizeType::SeamCarve => {
                transform::seam_carve(&mut self.0, op.width, op.height)
            }
        };
        self.0 = img;
    }
}

impl SpecTransform<&Watermark> for Photon {
    fn transform(&mut self, op: &Watermark) {
        multiple::watermark(&mut self.0, &WATERMARK, op.x, op.y);
    }
}

// fn image_to_buf(img: PhotonImage, format: ImageFormat) -> Vec<u8> {
//     let raw_pixels = img.get_raw_pixels();
//     let width = img.get_width();
//     let height = img.get_height();
//     let img_buffer = ImageBuffer::from_raw(width, height, raw_pixels).unwrap();
//     let dynimage = DynamicImage::ImageRgba8(img_buffer);
//     let mut buffer = Vec::with_capacity(32768);
//     dynimage.write_to(&mut buffer, format).unwrap();
//     buffer
// }

fn image_to_buf(img: PhotonImage, format: ImageFormat) -> Vec<u8> {
    // let raw_pixels = img.get_raw_pixels();
    // let width = img.get_width();
    // let height = img.get_height();

    // let img_buffer = ImageBuffer::from_vec(width, height, raw_pixels).unwrap();
    // let dynimage = DynamicImage::ImageRgb8(img_buffer);

    // let mut buffer = Vec::with_capacity(32768);
    // let mut cursor = Cursor::new(&mut buffer);
    // dynimage.write_to(&mut cursor, format).unwrap();
    // buffer

    let raw_pixels = img.get_raw_pixels();
    let width = img.get_width();
    let height = img.get_height();

    // 使用 ImageBuffer::from_raw 创建 RGBA 格式的 ImageBuffer
    let img_buffer = ImageBuffer::from_raw(width, height, raw_pixels).unwrap();
    let dynimage = DynamicImage::ImageRgba8(img_buffer);

    let mut buffer = Vec::with_capacity(32768);
    let mut cursor = Cursor::new(&mut buffer);
    dynimage.write_to(&mut cursor, format).unwrap();
    buffer
}
