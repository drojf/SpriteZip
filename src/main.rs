//std crates
extern crate core;
extern crate time;

//external crates
extern crate image;
extern crate brotli;
extern crate walkdir;

//standard uses
use std::io::{Write};
use std::fs::File;
use time::PreciseTime;

//non-standard use
use image::{RgbaImage, GenericImage};
use walkdir::WalkDir;

// TODO: crop diff'd images  so that not so much data needs to be compressed?

/*fn SubtractTwoImages(img1_dyn : &image::DynamicImage, img2_dyn : &image::DynamicImage, debug_mode : bool)
{
}*/

fn main() {
    let canvas_width = 3000;
    let canvas_height = 3000;

    let debug_mode = true;

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
        println!("\nProcessing image {}", count);

        let ent = entry.unwrap();

        let file_name_no_ext = ent.path().file_stem().unwrap().to_str().unwrap();//ent.path();

        if ent.file_type().is_dir() {
            continue;
        }

        let save_path = [file_name_no_ext, ".png"].concat();
        println!("Will save image to: {}", save_path);

        let img_dyn = image::open(ent.path()).unwrap();
        let img = img_dyn.as_rgba8().unwrap();

        println!("{}", ent.path().display());

        //subtract the image
        for (x, y, pixel) in img.enumerate_pixels()
        {
            let mut canvas_pixel = canvas.get_pixel_mut(x,y);

            //TODO: disable debug mode to use alpha value
            //must specify u8 to ensure wrapping occurs
            let new_pixel : [u8; 4] = [
                pixel[0].wrapping_sub(canvas_pixel[0]),
                pixel[1].wrapping_sub(canvas_pixel[1]),
                pixel[2].wrapping_sub(canvas_pixel[2]),
                if debug_mode {255} else {pixel[3].wrapping_sub(canvas_pixel[3])},
            ];

            *canvas_pixel = image::Rgba(new_pixel);
        }

        //save diff image as png for debugging reasons
        canvas.save(save_path).unwrap();

        // Compress the the diff image (or 'normal' image for first image)
        // NOTE: the below 'into_raw()' causes a move, so the canvas cannot be used anymore
        // However subsequent RgbaImage::new assigns a new value to the canvas each iteration
        {
            let canvas_as_raw = canvas.into_raw();

            let brotli_start = PreciseTime::now();
            compressor.write(&canvas_as_raw).unwrap();
            let brotli_end = PreciseTime::now();
            println!("Brotli compression took {} seconds", brotli_start.to(brotli_end));
        }

        //clear canvas (there must be a better way to do this?
        canvas = RgbaImage::new(canvas_width, canvas_height);

        //copy the originalimage onto canvas for next iteration
        canvas.copy_from(img, 0, 0);


        count += 1;
    }

}
