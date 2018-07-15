use std;
use std::fs;
use std::path::Path;
use std::io::{Read, Write, SeekFrom, Seek};
use std::default::Default;

use brotli;
use bincode;
use image;
use image::{RgbaImage};
use oxipng;
use png;
use png::HasParameters;

use common::{pretty_print_bytes};
use common::{DecompressionInfo};
use common::u8_buf_to_u64_little_endian;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::get_offset_to_other_image;
use common::BlockXYIterator;
use common::try_get_pixel;

pub fn extract_archive_alt(brotli_archive_path : &str, oxipng_options : Option<oxipng::Options>, debug_mode : bool) {
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
    brotli_file.read_exact(&mut compressed_bitmap).unwrap();
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
    let num_images = decompression_info.images_info.len();
    for (img_i, metadata) in decompression_info.images_info.into_iter().enumerate()
    {
        print!("{}/{}: ", img_i + 1, num_images);
        print!("diff: ({:4},{:4}) ", metadata.diff_width, metadata.diff_height);
        print!("full: ({:4},{:4}) ", metadata.output_width, metadata.output_height);
        print!("{}", metadata.output_path);
        println!("");

        //take a slice which contains only the desired region
        //read out the required number of bytes
        let expected_cropped_bitmap_size = metadata.diff_width as usize * metadata.diff_height as usize;
        if debug_mode { println!("meta: {:?}", metadata); println!("Trying to extract {} bytes of bitmap data", expected_cropped_bitmap_size)}

        let mut cropped_bitmap = vec![0u8; expected_cropped_bitmap_size];
        bitmap_info_decompressor.read_exact(&mut cropped_bitmap).unwrap();

        //reconstruct the image
        let mut full_image = RgbaImage::new(metadata.output_width, metadata.output_height);

        let (x_offset_to_prev_image , y_offset_to_prev_image)= get_offset_to_other_image(&full_image, &prev_image);
        if debug_mode { println!("Offset to other image: ({},{})", x_offset_to_prev_image , y_offset_to_prev_image); }

        //copy over the original image TODO: find a better way to do this?
        for y in 0..full_image.height() {
            for x in 0..full_image.width() {
                match try_get_pixel((x as i64 + x_offset_to_prev_image, y as i64 + y_offset_to_prev_image), &prev_image) {
                    None => {}
                    Some(pixel) => *full_image.get_pixel_mut(x, y) = pixel
                }
            }
        }

        //copy pixels which were different in the new image
        let mut pixel_count = 0;
        for (x,y) in BlockXYIterator::new(50, (metadata.diff_width as usize, metadata.diff_height as usize)) {

            let full_image_x = x + metadata.x;
            let full_image_y = y + metadata.y;

            //pixels are different - decompress a pixel from the compressed image data
            if cropped_bitmap[pixel_count] == 1 {
                let mut pixel_raw_data = [0u8; 4];
                image_data_decompressor.read_exact(&mut pixel_raw_data).unwrap();
                *full_image.get_pixel_mut(full_image_x, full_image_y) = image::Rgba::<u8>(pixel_raw_data);
            }

            pixel_count += 1;
        }

        //create the folder(s) to put the image in, then save the image
        let output_image_path = Path::new("output_images").join(&metadata.output_path);
        fs::create_dir_all(output_image_path.parent().unwrap()).unwrap();

        match &oxipng_options {
            None => full_image.save(output_image_path).unwrap(),
            Some(oxipng_options) => {
                //TODO: oxipng doesn't seem to accept raw images - only png images.
                //      in the future see if accept raw images, to avoid double compression/decompression
                let mut unoptimized_png_in_memory = Vec::new(); //TODO:at least size of current image - prevent reallocation
                {
                    let ref mut w = std::io::BufWriter::new(&mut unoptimized_png_in_memory);
                    let mut encoder = png::Encoder::new(w, full_image.width(), full_image.height()); // Width is 2 pixels and height is 1.
                    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
                    let mut writer = encoder.write_header().unwrap();

                    writer.write_image_data(&full_image.clone().into_raw()).unwrap(); // Save
                }

                let optimized_png = oxipng::optimize_from_memory(&unoptimized_png_in_memory[..], &oxipng_options).unwrap();
                std::fs::write(&output_image_path, &optimized_png[..]).unwrap();
            },
        }

        prev_image = full_image;
    }
}
