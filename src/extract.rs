use std::fs;
use std::path::{Path};
use std::io::Read; //needed to use brotli read trait

use brotli;
use serde_json;
use image::{imageops, RgbaImage, GenericImage};

use common::CANVAS_SETTING;
use common::CompressedImageInfo;
use common::add_image_to_canvas;
use common::offset_to_bottom_center_image_value;

pub fn extract_archive(brotli_archive_path : &str, metadata_path : &str, debug_mode : bool) {
    //unserialize the metadata file
    let metadata_file = fs::File::open(metadata_path).unwrap();
    let metadata_list: Vec<CompressedImageInfo> = serde_json::from_reader(metadata_file).unwrap();

    //open the brotli file for reading
    let brotli_file = fs::File::open(brotli_archive_path).unwrap();
    let mut extractor = brotli::Decompressor::new(
    brotli_file,
    CANVAS_SETTING.brotli_buffer_size);

    //initialize the canvas
    let mut canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

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

        if debug_mode { diff_image.save(format!("{}.png", count)).unwrap(); }

        //add the diff to the canvas at the specified coordinates
        add_image_to_canvas(&mut canvas, &diff_image, metadata.x, metadata.y);
        //debug: save entire canvas to preview reconstruction
        if debug_mode { canvas.save(format!("{}.canvas.png", count)).unwrap(); }

        //calculate
        let (crop_image_x, crop_image_y) = offset_to_bottom_center_image_value((canvas.width(), canvas.height()), (metadata.output_width, metadata.output_height));

        let reconstructed_image = imageops::crop(&mut canvas,
                  crop_image_x, crop_image_y,
                  metadata.output_width, metadata.output_height).to_image();

        //create the folder(s) to put the image in, then save the image
        let output_image_path = Path::new("output_images").join(metadata.output_path);
        fs::create_dir_all(output_image_path.parent().unwrap()).unwrap();
        reconstructed_image.save(output_image_path).unwrap();

        //clear the canvas
        canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

        //copy the reconstructed image onto the canvas?
        canvas.copy_from(&reconstructed_image, crop_image_x, crop_image_y);

        //get the correct crop of the canvas (using metadata) as a new image
        //save the reconstructed image as .png file
        count += 1;
    }
}