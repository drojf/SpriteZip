//standard uses
use std;
use std::io::{Write};
use std::fs;
use std::fs::File;
use time;
use std::path::{Path};

//non-standard use
use serde_json;
use brotli;
use image;
use image::{RgbaImage, GenericImage};
use walkdir::WalkDir;

use common::CompressedImageInfo;
use common::CANVAS_SETTING;
use common::subtract_image_from_canvas;
use common::offset_to_bottom_center_image;


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
fn crop_function(img: &image::RgbaImage, x_offset : u32, y_offset : u32, max_width : u32, max_height : u32) -> CroppedImageBounds
{
    //TODO: figure out a better rusty way to do this
    let mut x0 = x_offset + (max_width-1); //img.width();
    let mut x1 = x_offset; //std::u32::MAX;
    let mut y0 = y_offset + (max_height-1); //img.height();
    let mut y1 = y_offset; //std::u32::MAX;

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
fn save_brotli_image<T>(compressor : &mut brotli::CompressorWriter<T>, canvas_as_raw : &Vec<u8>)
where T: std::io::Write
{
    let brotli_start = time::PreciseTime::now();
    compressor.write(canvas_as_raw).unwrap();
    let brotli_end = time::PreciseTime::now();
    println!("Brotli compression took {} seconds", brotli_start.to(brotli_end));
}



//output_basename is the name of the brotli/metadatafiles, without the file extension (eg "a" will produce "a.brotli" and "a.metadata"
pub fn compress_path(brotli_archive_path : &str, metadata_path : &str, debug_mode : bool)
{
    let mut current_start_index : usize = 0;

    let mut images_meta_info = Vec::new();

    let crop_enabled = true;
    if debug_mode {
        println!("-----------
Warning: Debug mode is enabled - alpha channel
will be forced to 255 for .png output
-----------
    ");
    }

    let f = File::create(brotli_archive_path).expect("Cannot create file");

    let mut compressor = brotli::CompressorWriter::new(
    f,
    CANVAS_SETTING.brotli_buffer_size,
    CANVAS_SETTING.brotli_quality,
    CANVAS_SETTING.brotli_window);

    let mut canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

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

        //TODO: check if input image is larger than the canvas

        //Calculate image offset such that image is placed at the center bottom of the canvas.
        let (x_offset, y_offset) = offset_to_bottom_center_image(&canvas, &img);

        println!("Subtracting images");
        subtract_image_from_canvas(&mut canvas, &img, x_offset, y_offset);

        //TODO: crop diff
        let cropped_image_bounds = crop_function(&canvas,
                                                     x_offset, y_offset,
                                                     img.width(), img.height());

        //Note: a copy occurs here, for simplicity, so that the cropped image can be saved/compressed
        // As the cropped diff is usually small, this shouldn't have much impact on performance
        let cropped_image = if crop_enabled
        {
            image::imageops::crop(&mut canvas,
                              cropped_image_bounds.x, cropped_image_bounds.y,
                              cropped_image_bounds.width, cropped_image_bounds.height).to_image()
        }
        else
        {
            canvas.clone()
        };

        //save meta info
        let cropped_image_size = cropped_image.len();

        images_meta_info.push(CompressedImageInfo{
            start_index: current_start_index,   //where in the compressed data stream the image starts
            x: cropped_image_bounds.x,             //where on the canvas the diff should be placed (NEEDS UPDATE
            y: cropped_image_bounds.y,            //(NEEDS UPDATE
            diff_width: cropped_image_bounds.width,     //NEEDS UPDATE the width and height of the diff image
            diff_height: cropped_image_bounds.height,  // NEEDS UPDATE
            output_width: img.width(),   //the width and height of the reconstructed image
            output_height: img.height(),
            output_path: String::from(path_relative_to_input_folder),
        });

        current_start_index += cropped_image_size;

        println!("Image size is {},  width is {} height is {}", cropped_image_size, cropped_image_bounds.width, cropped_image_bounds.height);

        //save diff image as png for debugging reasons
        println!("Saving .png");
        if debug_mode
        {
            let mut cropped_image_copy = cropped_image.clone();
            for pixel in cropped_image_copy.pixels_mut()
            {
                *pixel = image::Rgba([
                    pixel[0],
                    pixel[1],
                    pixel[2],
                    255
                ]);
            }

            let save_path = Path::new("debug_images").join(path_relative_to_input_folder);
            println!("Will save image to: {}", save_path.to_str().unwrap());
            cropped_image_copy.save(save_path).unwrap()
        }

        // Compress the the diff image (or 'normal' image for first image)
        // NOTE: the below 'into_raw()' causes a move, so the canvas cannot be used anymore
        // However subsequent RgbaImage::new assigns a new value to the canvas each iteration
        let cropped_image_as_raw = cropped_image.into_raw();

        println!("Saving .brotli");
        save_brotli_image(&mut compressor, &cropped_image_as_raw);

        //clear canvas (there must be a better way to do this?
        canvas = RgbaImage::new(CANVAS_SETTING.width, CANVAS_SETTING.height);

        //copy the original image onto canvas for next iteration
        canvas.copy_from(img, x_offset, y_offset);


        count += 1;
    }

    //saving meta info
    let serialized = serde_json::to_string(&images_meta_info).unwrap();
    println!("serialized = {}", serialized);
    fs::write(metadata_path, serialized).expect("Unable to write metadata file");
}