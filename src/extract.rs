use std::fs;
use std::path::Path;
use std::io::Read; //needed to use brotli read trait
use std::io::SeekFrom;
use std::io::Seek;


use brotli;
use serde_json;
use bincode;
use image;
use image::{imageops, RgbaImage, GenericImage};

use common::{pretty_print_bytes, pretty_print_percent};
use common::{DecompressionInfo};
use common::add_image_to_canvas;
use common::offset_to_bottom_center_image_value;
use common::u8_buf_to_u64_little_endian;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::{convert_channel_based_to_pixel_based};
use common::get_offset_to_other_image;
use common::BlockXYIterator;
use common::try_get_pixel;

//

pub fn extract_archive_alt(brotli_archive_path : &str, debug_mode : bool) {
    //open the brotli file for reading
    let mut brotli_file = fs::File::open(brotli_archive_path).unwrap();

    //determine where the decompression info starts
    let mut header : [u8; FILE_FORMAT_HEADER_LENGTH] = [0; FILE_FORMAT_HEADER_LENGTH];
    brotli_file.read(&mut header).expect("Unable to read brotli file header");
    let decompression_info_start = u8_buf_to_u64_little_endian(&header);

    //Skip to the decompression information section, and deserialize
    let debug_metadata_start = brotli_file.seek(SeekFrom::Start(decompression_info_start)).unwrap();
    let decompression_info : DecompressionInfo = {
        let mut decompression_info_decompressor = brotli::Decompressor::new(&brotli_file, BROTLI_BUFFER_SIZE);
        let mut raw_decompression_info = Vec::new();
        decompression_info_decompressor.read_to_end(&mut raw_decompression_info).unwrap();
        bincode::deserialize(&raw_decompression_info).unwrap()
    };

    //fully read the compressed bitmap info into memory (theoretically can be avoided, but just do it this way for now...)
    //the data starts at decompression_info.bitmap_data_start and ends at (decompression_info_start-1)
    let mut compressed_bitmap = vec![0u8; decompression_info_start as usize - decompression_info.bitmap_data_start as usize];
    let debug_bitmap_start = brotli_file.seek(SeekFrom::Start(decompression_info.bitmap_data_start)).unwrap();
    brotli_file.read_exact(&mut compressed_bitmap);
    let mut bitmap_info_decompressor = brotli::Decompressor::new(&compressed_bitmap[..], BROTLI_BUFFER_SIZE);

    //Skip to the brotli compressed data section, then begin extraction
    let debug_image_data_start = brotli_file.seek(SeekFrom::Start(FILE_FORMAT_HEADER_LENGTH as u64)).unwrap();
    let mut image_data_decompressor = brotli::Decompressor::new(brotli_file, BROTLI_BUFFER_SIZE);

    println!("Brotli compressed data starts at {} ({}) [size: {}]",
             debug_image_data_start,
             pretty_print_bytes(debug_image_data_start as f64),
             pretty_print_bytes(debug_image_data_start as f64));
    println!("Bitmap information starts at {} ({}) [size: {}]",
             debug_bitmap_start,
             pretty_print_bytes(debug_bitmap_start as f64),
             pretty_print_bytes((debug_bitmap_start - debug_image_data_start) as f64));
    println!("Decompression information starts at {} ({}) [size: {}]",
             debug_metadata_start,
             pretty_print_bytes(debug_metadata_start as f64),
             pretty_print_bytes((debug_metadata_start - debug_bitmap_start) as f64),
    );
    println!("\n\n --------- Preparation Complete. Extracting Images ----------");

    //for each image
    let mut prev_image = RgbaImage::new(0,0); //on first image iteration, this should never get accessed

    for (img_count, metadata) in decompression_info.images_info.into_iter().enumerate()
    {
        println!("Extracting Image {} - meta: {:?}", img_count, metadata);

        //take a slice which contains only the desired region
        //read out the required number of bytes
        let expected_cropped_bitmap_size = metadata.diff_width as usize * metadata.diff_height as usize;
        println!("expected size: {}", expected_cropped_bitmap_size as f64 );

        let mut cropped_bitmap = vec![0u8; expected_cropped_bitmap_size];
        bitmap_info_decompressor.read_exact(&mut cropped_bitmap).unwrap();

        //reconstruct the image
        println!("Reconstructing Imgae...");
        let mut full_image = RgbaImage::new(metadata.output_width, metadata.output_height);

        let (x_offset_to_prev_image , y_offset_to_prev_image)= get_offset_to_other_image(&full_image, &prev_image);
        println!("Offset to other image: ({},{})", x_offset_to_prev_image , y_offset_to_prev_image);

        //copy over the original image TODO: find a better way to do this?
        for y in 0..full_image.height() {
            for x in 0..full_image.width() {
                match try_get_pixel((x as i64 + x_offset_to_prev_image, y as i64 + y_offset_to_prev_image), &prev_image) {
                    None => {}
                    Some(pixel) => *full_image.get_pixel_mut(x, y) = pixel
                }
            }
        }

        //
        let mut pixel_count = 0;
        for (x,y) in BlockXYIterator::new(50, (metadata.diff_width as usize, metadata.diff_height as usize)) {

            let full_image_x = x + metadata.x;
            let full_image_y = y + metadata.y;

            if x >= metadata.diff_width{
                println!("Error - x is out of range {}", x);
                return;
            }
            else if y >= metadata.diff_height  {
                println!("Error - y out of range {}", y);
                return;
            }
            else if pixel_count >= cropped_bitmap.len() {
                println!("Error - bitmap out of range {}", pixel_count);
                println!("Bitmap size is actually {}", cropped_bitmap.len());
                return;
            }

            *full_image.get_pixel_mut(full_image_x, full_image_y) = if cropped_bitmap[pixel_count] == 0 {
                //pixels are the same - use previous image's pixel at this coordinate
                let prev_image_x = (full_image_x as i64 + x_offset_to_prev_image) as u32;
                let prev_image_y = (full_image_y as i64 + y_offset_to_prev_image) as u32;

                *prev_image.get_pixel(prev_image_x, prev_image_y)
                //image::Rgba::<u8>([0u8; 4])
            }
            else
            {
//*full_image.get_pixel_mut(full_image_x, full_image_y) = {
//                    pixels are different - decompress a pixel from the compressed image data
                    let mut pixel_raw_data = [0u8; 4];
                    image_data_decompressor.read(&mut pixel_raw_data);
                    image::Rgba::<u8>(pixel_raw_data)
                    //  image::Rgba::<u8>([0u8; 4])
            };

            pixel_count += 1;
        }

        //create the folder(s) to put the image in, then save the image
        let output_image_path = Path::new("output_images").join(&metadata.output_path);
        fs::create_dir_all(output_image_path.parent().unwrap()).unwrap();
        full_image.save(output_image_path).unwrap();

        println!("");

        prev_image = full_image;
    }
}

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