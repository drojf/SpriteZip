extern crate brotli;
extern crate image;
extern crate core;

//use std::io;
use std::io::{Write, Read};
use std::fs::File;
use core::slice::Iter;


use image::{GenericImage, imageops};

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


fn main() {

    let debug_mode = true;

    if false
    {
        compression_test();
    }

    // Use the open function to load an image from a Path.
    // ```open``` returns a `DynamicImage` on success.
    println!("Load img1");
    let mut img1_dyn = image::open("1.png").unwrap();
    println!("Load img2");
    let mut img2_dyn = image::open("2.png").unwrap();

    let img1 = img1_dyn.as_mut_rgba8().unwrap();
    let img2 = img2_dyn.as_mut_rgba8().unwrap();

    {
        //let subimage = imageops::crop(&mut img, 0, 0, 50, 50);
        //let mut testimage = img.to_rgba();
        //let mut testimage = img1.as_mut_rgba8().unwrap();
        //let mut img2mut =
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

        println!("Saving file");
        img1.save("test2.png").unwrap();
        println!("Finished.");
        compress_data(img1.iter());
    }

    // The dimensions method returns the images width and height.
    //println!("dimensions {:?}", img.dimensions());

    // The color method returns the image's `ColorType`.
    //println!("{:?}", img.color());

    // Write the contents of this image to the Writer in PNG format.
    //img.save("test2.png").unwrap();

}
