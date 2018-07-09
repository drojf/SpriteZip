//standard uses
use std;
use std::io::{Write};
use std::fs::File;
use time;
use std::io::SeekFrom;
use std::io::Seek;

//non-standard use
use serde_json;
use bincode;

use brotli;
use image;
use image::{RgbaImage, GenericImage};
use walkdir::WalkDir;

use common::{CompressedImageInfo, DecompressionInfo};
use common::subtract_image_from_canvas;
use common::offset_to_bottom_center_image;
use common::scan_folder_for_max_png_size;
use common::u64_to_u8_buf_little_endian;
use common::save_image_no_alpha;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::{convert_pixel_based_to_channel_based};

struct CroppedImageBounds {
    x : u32,
    y : u32,
    width : u32,
    height : u32,
}

//TODO: cropped image cannot be bigger than two input images - can optimize for this
/// This function takes in an image, and returns the bounding box of the image.
/// It assumes that pixels whose components are all 0 are not important. All other
/// pixels will be counted when calculating the bounding box
/// img: image to find the bounding box
/// x_offset, y_offset: within the image, the x and y offset to begin searching for the bounding box
/// max_width, max height: the max size to search for the bounding box within the image, starting from
/// the x and y offset
/// (the x and y offset and max width/height let you specify a subimage in which to find the bounding box of the image)
/// return value: the bounding box of the image (given as offset & size). Note that the returned
/// coordinates are in absolute coordinates, not relative to the x_offset and y_offset input values
fn crop_function(img: &image::RgbaImage, offset : (u32, u32), max_width : u32, max_height : u32) -> CroppedImageBounds
{
    //TODO: figure out a better rusty way to do this
    let mut x0 = offset.0 + (max_width-1);
    let mut x1 = offset.0;
    let mut y0 = offset.1 + (max_height-1);
    let mut y1 = offset.1;

    for (x, y, pixel) in img.enumerate_pixels()
    {
        if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 || pixel[3] != 0
        {
            x0 = std::cmp::min(x, x0);
            y0 = std::cmp::min(y, y0);
            x1 = std::cmp::max(x, x1);
            y1 = std::cmp::max(y, y1);
        }
    }

    CroppedImageBounds {
        x: x0, y: y0,
        width: x1-x0+1, height: y1-y0+1,
    }
}

/// Saves a raw canvas image using an external compressor
/// compressor: the compressor to use to save the image
/// canvas_as_raw: the raw image to be saved using the compressor
fn save_brotli_image<T>(compressor : &mut brotli::CompressorWriter<T>, canvas_as_raw : &Vec<u8>, print_execution_time : bool) -> usize
where T: std::io::Write
{
    let brotli_start = time::PreciseTime::now();
    let bytes_written = compressor.write(canvas_as_raw).unwrap();
    let brotli_end = time::PreciseTime::now();
    if print_execution_time { println!("Brotli compression took {} seconds", brotli_start.to(brotli_end)); }
    return bytes_written;
}

/// File format is as follows:
/// [8 bytes BigE 64u = X]  - a Big Endian, 64u value indicating the length of the CompressedFileInfo JSON at the end of the file
/// [Variable Length]       -  Brotli compressed image data.
/// [X bytes long]          - JSON encoded DecompressionInfo struct. Length given in first 4 bytes of the file.
//output_basename is the name of the brotli/metadatafiles, without the file extension (eg "a" will produce "a.brotli" and "a.metadata"
//Note: json is always stored uncompressed!
pub fn compress_path(brotli_archive_path : &str, use_json : bool, debug_mode : bool)
{
    let brotli_quality = 11;
    let brotli_window = 24;

    let (max_width, max_height) = scan_folder_for_max_png_size("input_images");
    let canvas_width  = (max_width + 1)  & !0x1;
    let canvas_height = (max_height + 1) & !0x1;
    println!("max image size: ({},{})\nchosen canvas size: ({},{})", max_width, max_height, canvas_width, canvas_height);

    let mut image_start_index_in_brotli_stream : usize = 0;
    let mut images_meta_info = Vec::new();
    let mut brotli_file = File::create(brotli_archive_path).expect("Cannot create file");

    //allocate some space to record how long the brotli compressed data is
    brotli_file.write(&[0; FILE_FORMAT_HEADER_LENGTH]).expect("Unable to allocate header space in file");

    //This set of braces forces the &f mutable reference used in the 'compressor' to go out of scope
    {
        let mut compressor = brotli::CompressorWriter::new(
        &brotli_file,
        BROTLI_BUFFER_SIZE,
        brotli_quality,
        brotli_window);

        let mut canvas = RgbaImage::new(canvas_width, canvas_height);

        println!("Begin scanning for images");

        //TODO: check that input_images directory exists before scanning it.
        //TODO: check each image's color type as subtracting a RGB image from an RGBA image shouldn't work.
        // see: println!("{:?}", img.color());

        let test_iter = WalkDir::new("input_images");
        let mut count = 0;
        for entry in test_iter
        {
            let ent = entry.unwrap();
            if ent.file_type().is_dir() {
                continue;
            }

            let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();

            println!("\nProcessing Image {}: '{}'", count, ent.path().display());
            let img_dyn = image::open(ent.path()).unwrap();
            let img = img_dyn.as_rgba8().unwrap();
            println!("Original Image width is {} height is {}", img.width(), img.height());

            if debug_mode { println!("Subtracting image from bottom center of canvas, then cropping image "); }
            let offset_from_bottom_center = offset_to_bottom_center_image(&canvas, &img);
            subtract_image_from_canvas(&mut canvas, &img, offset_from_bottom_center);

            let cropped_image_bounds =
                crop_function(&canvas, offset_from_bottom_center, img.width(), img.height());

            let cropped_image =
                image::imageops::crop(&mut canvas,
                cropped_image_bounds.x, cropped_image_bounds.y,
                cropped_image_bounds.width, cropped_image_bounds.height).to_image();

            //save meta info
            images_meta_info.push(CompressedImageInfo{
                start_index: image_start_index_in_brotli_stream,   //where in the compressed data stream the image starts
                x: cropped_image_bounds.x,                         //where on the canvas the diff should be placed (NEEDS UPDATE
                y: cropped_image_bounds.y,
                diff_width: cropped_image_bounds.width,            //the width and height of the diff image
                diff_height: cropped_image_bounds.height,
                output_width: img.width(),                         //the width and height of the reconstructed image
                output_height: img.height(),
                output_path: String::from(path_relative_to_input_folder),
            });

            image_start_index_in_brotli_stream += cropped_image.len();
            println!("Image size is {},  width is {} height is {}", cropped_image.len(), cropped_image_bounds.width, cropped_image_bounds.height);

            //save diff image as png for debugging reasons
            if debug_mode { save_image_no_alpha(cropped_image.clone(), &path_relative_to_input_folder); }

            // Compress the the diff image (or 'normal' image for first image)
            // NOTE: the below 'into_raw()' causes a move, so the canvas cannot be used anymore
            // However subsequent RgbaImage::new assigns a new value to the canvas each iteration
            let cropped_image_as_raw = cropped_image.into_raw();
            save_brotli_image(&mut compressor,
                              &convert_pixel_based_to_channel_based(cropped_image_as_raw, (cropped_image_bounds.width, cropped_image_bounds.height)),
                              true);

            // Prepare for next iteration by clearing canvas, then copying the 'original' image for the next diff
            canvas = RgbaImage::new(canvas_width, canvas_height);
            canvas.copy_from(img, offset_from_bottom_center.0, offset_from_bottom_center.1);


            count += 1;
        }
    }

    //Save decompression info to file, and record its length
    let decompression_info = DecompressionInfo {
        canvas_size: (canvas_width, canvas_height),
        images_info: images_meta_info,
    };

    //saving meta info
    let decompression_info_start = brotli_file.seek(SeekFrom::Current(0)).unwrap();
    println!("Decompression Info starts at position {}", decompression_info_start);

    let serialized : Vec<u8> =  if use_json {
        serde_json::to_vec(&decompression_info).unwrap()
    } else {
        bincode::serialize(&decompression_info).unwrap()
    };

    if use_json {
        brotli_file.write(&serialized).expect("Unable to write metadata file");
    }
    else {
        let mut compressor = brotli::CompressorWriter::new(
            &brotli_file,
            BROTLI_BUFFER_SIZE,
            brotli_quality,
            brotli_window);
        compressor.write(&serialized).expect("Unable to write decompression info (brotli compressed)");
    }

    //save some information about the compression before writing header at start of file
    let uncompressed_metadata_size =  serialized.len();
    let total_file_size = brotli_file.seek(SeekFrom::Current(0)).unwrap();
    let metadata_length_bytes = total_file_size - decompression_info_start;
    let metadata_as_percentage_of_total = metadata_length_bytes as f64 / total_file_size as f64 * 100.0;

    //return to start of file to write header info
    brotli_file.seek(SeekFrom::Start(0)).unwrap();
    brotli_file.write(&u64_to_u8_buf_little_endian(decompression_info_start)).expect("Unable to write header info to file");

    println!("\n\n ------------ Compression Finished! ------------");
    println!("Total archive size is {}mbytes", total_file_size as f64 / 1e6);
    println!("Metadata is {} kbytes ({} uncompressed), {}% of total",
             metadata_length_bytes as f64 / 1000.0,
             uncompressed_metadata_size as f64 / 1000.0,
             metadata_as_percentage_of_total);
}