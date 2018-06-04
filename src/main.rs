extern crate brotli;
//use std::io;
use std::io::{Write, Read};
use std::fs::File;

struct PrintAsDecimal {

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
}

fn main() {
    let filename = "compressed.brotli";
    let mut buf = [0u8; 4096];

    for x in 0..4096
    {
        buf[x] = x as u8;
    }


        let simple_output = PrintAsDecimal {};
/*
    //let stdout = &mut io::stdout();
    let mut writer = brotli::CompressorWriter::new(
    simple_output,
    4096,
    11,
22);


    writer.write_all(&buf).unwrap();*/



    //write compressed data to file
/*    let mut f = File::create(filename).expect("Cannot create file");
    let mut writer = brotli::CompressorWriter::new(
    f,
    4096,
    11,
    22);

    writer.write(&buf).unwrap();*/

    //read compressed file
    let mut f = File::open(filename).expect("Cannot open file");
    let mut reader = brotli::Decompressor::new(
    f,
    4096);


    //reader.read(&simple_output);
    let mut buf = [0u8; 4096];
    reader.read(&mut buf);

    for x in buf.iter()
    {
        println!("{}", x);
    }



}
