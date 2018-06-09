extern crate brotli;
extern crate image;

//use std::io;
use std::io::{Write, Read};
use std::fs::File;

use image::GenericImage;

/*struct PrintAsDecimal {

}

impl Write for PrintAsDecimal {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        for x in buf
        {
            println!("{}", x);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

impl Read for PrintAsDecimal {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let size = buf.len();

        for x in buf
        {
            println!("{}", x);
        }

        Ok(size)
    }
}*/

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
    let img = image::open("test.png").unwrap();

    // The dimensions method returns the images width and height.
    println!("dimensions {:?}", img.dimensions());

    // The color method returns the image's `ColorType`.
    println!("{:?}", img.color());

    // Write the contents of this image to the Writer in PNG format.
    img.save("test2.png").unwrap();


}
