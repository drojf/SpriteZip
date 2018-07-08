use std::fs;
use std::path::Path;
use std::io::Read; //needed to use brotli read trait
use std::io::SeekFrom;
use std::io::Seek;


use brotli;
use serde_json;
use bincode;
use image::{imageops, RgbaImage, GenericImage};

use common::{DecompressionInfo};
use common::add_image_to_canvas;
use common::offset_to_bottom_center_image_value;
use common::u8_buf_to_u64_little_endian;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::{convert_channel_based_to_pixel_based};

pub fn extract_archive(brotli_archive_path : &str, use_json : bool, debug_mode : bool) {
    //open the brotli file for reading
    let mut brotli_file = fs::File::open(brotli_archive_path).unwrap();

    //determine where the decompression info starts
    let mut header : [u8; FILE_FORMAT_HEADER_LENGTH] = [0; FILE_FORMAT_HEADER_LENGTH];
    brotli_file.read(&mut header).expect("Unable to read brotli file header");
    let decompression_info_start = u8_buf_to_u64_little_endian(&header);

    //Skip to the decompression information section, and deserialize
    println!("Decompression information starts at {}",
        brotli_file.seek(SeekFrom::Start(decompression_info_start)).unwrap()
    );

    let decompression_info : DecompressionInfo = if use_json {
        serde_json::from_reader(&brotli_file).unwrap()
    } else {
        let mut decompression_info_decompressor = brotli::Decompressor::new(&brotli_file, BROTLI_BUFFER_SIZE);
        let mut raw_decompression_info = Vec::new();
        decompression_info_decompressor.read_to_end(&mut raw_decompression_info).unwrap();
        bincode::deserialize(&raw_decompression_info).unwrap()
    };

    let canvas_width = decompression_info.canvas_size.0;
    let canvas_height = decompression_info.canvas_size.1;
    let metadata_list = &decompression_info.images_info;

    //Skip to the brotli compressed data section, then begin extraction
    println!("Brotli compressed data starts at {}",
         brotli_file.seek(SeekFrom::Start(FILE_FORMAT_HEADER_LENGTH as u64)).unwrap()
    );

    let mut extractor = brotli::Decompressor::new(brotli_file, BROTLI_BUFFER_SIZE);

    //initialize the canvas
    let mut canvas = RgbaImage::new(canvas_width, canvas_height);

    //for each image
    let mut count = 0;
    let mut brotli_data_ptr = 0;
    for metadata in metadata_list
    {
        println!("Extracting Image {} - meta: {:?}", count, metadata);

        let expected_size = metadata.diff_width as usize * metadata.diff_height as usize * 4; //RGBA image = 4 byte/pixel
        brotli_data_ptr += expected_size;
        println!("expected size: {} expected next image start {}", expected_size, brotli_data_ptr);

        //partially decompress the brotli file
        let mut raw_image_data_channel_based = vec![0; expected_size];
        //TODO: add erorr check here? see https://doc.rust-lang.org/std/io/trait.Read.html
        extractor.read_exact(&mut raw_image_data_channel_based).unwrap();
        let mut raw_image_data = convert_channel_based_to_pixel_based(raw_image_data_channel_based, (metadata.diff_width, metadata.diff_height));

        //debug: interpret the raw data as an image and save to file
        let diff_image = RgbaImage::from_raw(metadata.diff_width, metadata.diff_height, raw_image_data).expect("diff image could not be created from raw image");

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
        let output_image_path = Path::new("output_images").join(&metadata.output_path);
        fs::create_dir_all(output_image_path.parent().unwrap()).unwrap();
        reconstructed_image.save(output_image_path).unwrap();

        //clear the canvas
        canvas = RgbaImage::new(canvas_width, canvas_height);

        //copy the reconstructed image onto the canvas?
        canvas.copy_from(&reconstructed_image, crop_image_x, crop_image_y);

        //get the correct crop of the canvas (using metadata) as a new image
        //save the reconstructed image as .png file
        count += 1;
    }
}