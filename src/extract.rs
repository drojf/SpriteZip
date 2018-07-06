use std::fs;

use std::io::Read; //needed to use brotli read trait

use brotli;
use serde_json;
use image::{RgbaImage, GenericImage};

use common::CANVAS_SETTING;
use common::CompressedImageInfo;

pub fn extract_archive(brotli_archive_path : &str, metadata_path : &str) {
    //unserialize the metadata file
    let metadata_file = fs::File::open(metadata_path).unwrap();
    let metadata_list: Vec<CompressedImageInfo> = serde_json::from_reader(metadata_file).unwrap();

    //open the brotli file for reading
    let brotli_file = fs::File::open(brotli_archive_path).unwrap();
    let mut extractor = brotli::Decompressor::new(
    brotli_file,
    CANVAS_SETTING.brotli_buffer_size);

    //initialize the canvas
    let canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

    //for each image
    let mut count = 0;
    let mut brotli_data_ptr = 0;
    for metadata in metadata_list
    {
        println!("{:?}", metadata);

        let expected_size = metadata.diff_width as usize * metadata.diff_height as usize * 4; //RGBA image = 4 byte/pixel
        brotli_data_ptr += expected_size;
        println!("expected size: {} expected next image start {}", expected_size, brotli_data_ptr);

        //partially decompress the brotli file
        let mut raw_image_data = vec![0; expected_size];
        //TODO: add erorr check here? see https://doc.rust-lang.org/std/io/trait.Read.html
        extractor.read_exact(&mut raw_image_data).unwrap();

        //debug: interpret the raw data as an image and save to file
        let diff_image = RgbaImage::from_raw(metadata.diff_width, metadata.diff_height, raw_image_data).unwrap();

        diff_image.save(format!("{}.png", count)).unwrap();


        //add the diff to the canvas at the specified coordinates
        //get the correct crop of the canvas (using metadata) as a new image
        //save the reconstructed image as .png file
        count += 1;
    }
}