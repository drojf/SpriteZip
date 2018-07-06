use std::fs;

use serde_json;
use image::{RgbaImage, GenericImage};

use common::CANVAS_SETTING;
use common::CompressedImageInfo;

pub fn extract_archive(brotli_archive_path : &str, metadata_path : &str) {
    let data = fs::read(metadata_path).expect("Unable to read metadata file");

    println!("Loaded metadata file: {}", data.len());

    //unserialize the metadata file
    let metadata_file = fs::File::open(metadata_path).unwrap();
    let metadata_list: Vec<CompressedImageInfo> = serde_json::from_reader(metadata_file).unwrap();

    //open the brotli file for reading
    let brotli_file = fs::File::open(brotli_archive_path).unwrap();

    //initialize the canvas
    let canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

    //for each image
    for metadata in metadata_list
    {
        println!("{:?}", metadata);
        //partially decompress the brotli file
        //add the diff to the canvas at the specified coordinates
        //get the correct crop of the canvas (using metadata) as a new image
        //save the reconstructed image as .png file
    }
}