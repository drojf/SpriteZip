extern crate brotli;
extern crate image;

//use std::io;
use std::io::{Write, Read};
use std::fs::File;

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

fn main() {
    if false
    {
        compression_test();
    }

    // Use the open function to load an image from a Path.
    // ```open``` returns a `DynamicImage` on success.
    let mut img = image::open("test.png").unwrap();
    
    {
        //let subimage = imageops::crop(&mut img, 0, 0, 50, 50);
        let mut testimage = img.to_rgba();
        for (x, y, pixel) in testimage.enumerate_pixels_mut() {
            *pixel = image::Rgba([
                0u8.wrapping_sub(pixel[0]),
                0u8.wrapping_sub(pixel[1]),
                0u8.wrapping_sub(pixel[2]),
                pixel[3],
            ]);
        }

        testimage.save("test2.png").unwrap();
    }
    // The dimensions method returns the images width and height.
    //println!("dimensions {:?}", img.dimensions());

    // The color method returns the image's `ColorType`.
    //println!("{:?}", img.color());

    // Write the contents of this image to the Writer in PNG format.
    //img.save("test2.png").unwrap();

}
