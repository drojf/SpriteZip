extern crate brotli;
extern crate image;
extern crate core;
extern crate time;
extern crate walkdir;

use std::io::{Write, Read};
use std::fs::File;
use core::slice::Iter;
use time::PreciseTime;


use image::{GenericImage, imageops};

use walkdir::WalkDir;

fn compression_test() -> ()
{
    let compress = false;

    let filename = "compressed.brotli";
    let mut buf = [0u8; 4096];

    for x in 0..4096
    {
        buf[x] = x as u8;
    }

    //write compressed data to file
    if compress
    {
        let f = File::create(filename).expect("Cannot create file");
        let mut writer = brotli::CompressorWriter::new(
            f,
            4096,
            11,
            22);

        writer.write(&buf).unwrap();
    }
    else
    {
        //read compressed file
        let f = File::open(filename).expect("Cannot open file");
        let mut reader = brotli::Decompressor::new(
        f,
        4096);


        //reader.read(&simple_output);
        let mut buf = [0u8; 4096];
        reader.read(&mut buf).unwrap();

        for x in buf.iter()
        {
            println!("{}", x);
        }
    }
}

//not sure if the iterator is better than feeding in the entire buffer,
//but it seems easier to pass the data around this way...
fn compress_data(iter : Iter<u8>)
{
    let filename = "compressed_image.brotli";

    let f = File::create(filename).expect("Cannot create file");
    let mut writer = brotli::CompressorWriter::new(
        f,
        4096,
        11,//11,
        22);

    for val in iter
    {
        writer.write(&[*val]).unwrap();
    }
}

fn SubtractTwoImages(img1_dyn : &image::DynamicImage, img2_dyn : &image::DynamicImage, debug_mode : bool)
{
       /* let img1 = img1_dyn.as_mut_rgba8().unwrap();
        let img2 = img2_dyn.as_mut_rgba8().unwrap();

        println!("Subtracting two images");
        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            let other_pixel = img2.get_pixel(x,y);

            *pixel = image::Rgba([
                other_pixel[0].wrapping_sub(pixel[0]),
                other_pixel[1].wrapping_sub(pixel[1]),
                other_pixel[2].wrapping_sub(pixel[2]),
                other_pixel[3].wrapping_sub(pixel[3]),
            ]);

            if debug_mode
            {
                pixel[3] = 255;
            }
        }*/
}

fn main() {
    let debug_mode = true;

    let file_name_no_ext = "compressed_image";
    let f = File::create([file_name_no_ext, ".brotli"].concat()).expect("Cannot create file");

    let mut compressor = brotli::CompressorWriter::new(
    f,
    4096,
    9,//11, //9 seems to be a good tradeoff...changing q doesn't seem to make much diff though?
    22);

    let mut img1_dyn_some = None;
    let mut img2_dyn_some = None;

    println!("Begin scanning for images");

    //TODO: check that input_images directory exists before scanning it.

    let test_iter = WalkDir::new("input_images");
    let mut count = 0;
    for entry in test_iter {

        let ent = entry.unwrap();

        if ent.file_type().is_dir() {
            continue;
        }

        if count == 0
        {
            println!("Scan Image1");
            println!("first_item: {}", ent.path().display());
            img1_dyn_some = Some(image::open(ent.path()).unwrap());
            //let mut img1_dyn = image::open("1.png").unwrap();
            //img1 = Some(img1_dyn.as_mut_rgba8().unwrap());
        }
        else
        {
            println!("Scan Image2");
            println!("{}", ent.path().display());
            img2_dyn_some = Some(image::open(ent.path()).unwrap());
            //let mut img2_dyn = image::open("2.png").unwrap();
            //img2 = Some(img2_dyn.as_mut_rgba8().unwrap());
            {
                //SubtractTwoImages(&img1.unwrap(), &img2.unwrap(), true);
            }
            //img1 = img2;
            let mut img1_dyn = img1_dyn_some.unwrap();
            let mut img2_dyn = img2_dyn_some.unwrap();

            let img1 = img1_dyn.as_mut_rgba8().unwrap();
            let img2 = img2_dyn.as_mut_rgba8().unwrap();

            println!("Subtracting two images");
            for (x, y, pixel) in img1.enumerate_pixels_mut() {
                let other_pixel = img2.get_pixel(x,y);

                *pixel = image::Rgba([
                    other_pixel[0].wrapping_sub(pixel[0]),
                    other_pixel[1].wrapping_sub(pixel[1]),
                    other_pixel[2].wrapping_sub(pixel[2]),
                    other_pixel[3].wrapping_sub(pixel[3]),
                ]);

                if debug_mode
                {
                    pixel[3] = 255;
                }
            }

        }
    }

    let file_name_no_ext = "compressed_image";


    let debug_mode = true;

    // Use the open function to load an image from a Path.
    // ```open``` returns a `DynamicImage` on success.
    /*println!("Load img1");
    let mut img1_dyn = image::open("1.png").unwrap();
    println!("Load img2");
    let mut img2_dyn = image::open("2.png").unwrap();

    {
        let img1 = img1_dyn.as_mut_rgba8().unwrap();
        let img2 = img2_dyn.as_mut_rgba8().unwrap();

        println!("Subtracting two images");
        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            let other_pixel = img2.get_pixel(x,y);

            *pixel = image::Rgba([
                other_pixel[0].wrapping_sub(pixel[0]),
                other_pixel[1].wrapping_sub(pixel[1]),
                other_pixel[2].wrapping_sub(pixel[2]),
                other_pixel[3].wrapping_sub(pixel[3]),
            ]);

            if debug_mode
            {
                pixel[3] = 255;
            }
        }

        println!("Saving .png");
        let png_start = PreciseTime::now();
        img1.save([file_name_no_ext, ".png"].concat()).unwrap();
        let png_end = PreciseTime::now();


        println!("Compressing...");
        let brotli_start = PreciseTime::now();
        for val in img1.iter()
        {
             compressor.write(&[*val]);
        }
        let brotli_end = PreciseTime::now();


        println!("Finished.");
        let png_time = png_start.to(png_end);
        let brotli_time = brotli_start.to(brotli_end);
        println!("PNG compression    took {} seconds", png_time);
        println!("Brotli compression took {} seconds", brotli_time);
        println!("Brotli is {} times slower", brotli_time.num_milliseconds() / png_time.num_milliseconds());

    }*/



    // The dimensions method returns the images width and height.
    //println!("dimensions {:?}", img.dimensions());

    // The color method returns the image's `ColorType`.
    //println!("{:?}", img.color());

    // Write the contents of this image to the Writer in PNG format.
    //img.save("test2.png").unwrap();

}
