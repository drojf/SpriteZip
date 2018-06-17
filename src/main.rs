//Note: please run with --release (or higher optimization level), otherwise running program is way too slow.

#[macro_use]
extern crate serde_derive;

//std crates
extern crate core;
extern crate time;

//external crates
extern crate image;
extern crate brotli;
extern crate walkdir;
extern crate serde;
extern crate serde_json;

//standard uses
use std::io::{Write};
use std::fs::File;
use time::PreciseTime;

//non-standard use
use image::{RgbaImage, GenericImage};
use walkdir::WalkDir;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct CompressedImageInfo {
    start_index: usize,   //where in the compressed data stream the image starts
    x: u32,             //where on the canvas the diff should be placed
    y: u32,
    diff_width: u32,     //the width and height of the diff image
    diff_height: u32,
    output_width: u32,   //the width and height of the reconstructed image
    output_height: u32,
}

/*fn calculate_offset_to_bottom_center_image(canvas: &mut image::RgbaImage, img : &image::RgbaImage) -> (i32, i32)
{
}*/

struct CroppedImageBounds {
    x : u32,
    y : u32,
    width : u32,
    height : u32,
}

/*fn placeholder_crop_function(img: &image::RgbaImage, x_offset : u32, y_offset : u32, max_width : u32, max_height : u32) -> CroppedImageBounds
{
    CroppedImageBounds {
        x: 0, y: 0,
        width: img.width(), height: img.height(),
    }
}*/

//TODO: cropped image cannot be bigger than two input images - can optimize for this
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

// TODO: crop diff'd images  so that not so much data needs to be compressed?
fn subtract_image_from_canvas(canvas: &mut image::RgbaImage, img : &image::RgbaImage, x_offset : u32, y_offset : u32)
{
    for (x, y, pixel) in img.enumerate_pixels()
    {
        let mut canvas_pixel = canvas.get_pixel_mut(x + x_offset, y + y_offset);

        //TODO: disable debug mode to use alpha value
        //must specify u8 to ensure wrapping occurs
        let new_pixel : [u8; 4] = [
            pixel[0].wrapping_sub(canvas_pixel[0]),
            pixel[1].wrapping_sub(canvas_pixel[1]),
            pixel[2].wrapping_sub(canvas_pixel[2]),
            pixel[3].wrapping_sub(canvas_pixel[3]),
        ];

        *canvas_pixel = image::Rgba(new_pixel);
    }
}

fn save_brotli_image<T>(compressor : &mut brotli::CompressorWriter<T>, canvas_as_raw : &Vec<u8>)
where T: std::io::Write
{
    let brotli_start = PreciseTime::now();
    compressor.write(canvas_as_raw).unwrap();
    let brotli_end = PreciseTime::now();
    println!("Brotli compression took {} seconds", brotli_start.to(brotli_end));
}

fn main() {
    let mut current_start_index : usize = 0;

    let mut images_meta_info = Vec::new();

    let canvas_width = 3000;
    let canvas_height = 3000;

    let debug_mode = true;
    if debug_mode {
        println!("-----------
Warning: Debug mode is enabled - alpha channel
will be forced to 255 for .png output
-----------
    ");
    }

    let f = File::create(["compressed_images", ".brotli"].concat()).expect("Cannot create file");

    let mut compressor = brotli::CompressorWriter::new(
    f,
    4096,
    9,//11, //9 seems to be a good tradeoff...changing q doesn't seem to make much diff though?
    22);

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

        println!("\nProcessing Image {}: '{}'", count, ent.path().display());

        let file_name_no_ext = ent.path().file_stem().unwrap().to_str().unwrap();
        let save_path = [file_name_no_ext, ".png"].concat();
        println!("Will save image to: {}", save_path);

        let img_dyn = image::open(ent.path()).unwrap();
        let img = img_dyn.as_rgba8().unwrap();

        println!("Original Image width is {} height is {}", img.width(), img.height());

        //TODO: check if input image is larger than the canvas

        //Calculate image offset such that image is placed at the center bottom of the canvas.
        let x_offset = (canvas.width() - img.width()) / 2;
        let y_offset = canvas.height() - img.height();

        println!("Subtracting images");
        subtract_image_from_canvas(&mut canvas, &img, x_offset, y_offset);

        //TODO: crop diff
/*        let cropped_image_bounds = placeholder_crop_function(&canvas,
                                                             x_offset, y_offset,
                                                             img.width(), img.height());*/

        let cropped_image_bounds = crop_function(&canvas,
                                                     x_offset, y_offset,
                                                     img.width(), img.height());

        //Note: a copy occurs here, for simplicity, so that the cropped image can be saved/compressed
        // As the cropped diff is usually small, this shouldn't have much impact on performance
        let cropped_image = image::imageops::crop(&mut canvas,
                              cropped_image_bounds.x, cropped_image_bounds.y,
                              cropped_image_bounds.width, cropped_image_bounds.height).to_image();

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

            cropped_image_copy.save(save_path).unwrap()
        }
        else
        {
            cropped_image.save(save_path).unwrap();
        }

        // Compress the the diff image (or 'normal' image for first image)
        // NOTE: the below 'into_raw()' causes a move, so the canvas cannot be used anymore
        // However subsequent RgbaImage::new assigns a new value to the canvas each iteration
        let canvas_as_raw = canvas.into_raw();

        println!("Saving .brotli");
        save_brotli_image(&mut compressor, &canvas_as_raw);

        //clear canvas (there must be a better way to do this?
        canvas = RgbaImage::new(canvas_width, canvas_height);

        //copy the original image onto canvas for next iteration
        canvas.copy_from(img, x_offset, y_offset);


        count += 1;
    }

    //saving meta info
    let serialized = serde_json::to_string(&images_meta_info).unwrap();
    println!("serialized = {}", serialized);


}
