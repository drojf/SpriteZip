use image;
use image::RgbaImage;

use walkdir;

use std;
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::io::{Read, Write};
use brotli;
use number_prefix::NumberPrefix;

pub const FILE_FORMAT_HEADER_LENGTH: usize = 8;
pub const BROTLI_BUFFER_SIZE: usize = 4096; //buffer size used for compression and decompression

pub fn get_offset_to_other_image(original_image : &image::RgbaImage, prev_image : &image::RgbaImage) -> (i64, i64)
{
    let prev_x_offset = (prev_image.width() as i64 - original_image.width()  as i64)/2;
    let prev_y_offset = prev_image.height() as i64 - original_image.height() as i64;
    (prev_x_offset, prev_y_offset)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecompressionInfo {
    pub canvas_size: (u32, u32),
    pub bitmap_data_start : u64,
    pub images_info:  Vec<CompressedImageInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompressedImageInfo {
    pub start_index: usize,
    //where in the compressed data stream the image starts
    pub x: u32,
    //where on the canvas the diff should be placed
    pub y: u32,
    pub diff_width: u32,
    //the width and height of the diff image
    pub diff_height: u32,
    pub output_width: u32,
    //the width and height of the reconstructed image
    pub output_height: u32,
    //the output path of the image
    pub output_path: String,
}

pub fn pretty_print_bytes(value : f64) -> String
{
    match NumberPrefix::decimal(value as f64) {
        NumberPrefix::Standalone(bytes)   => format!("{} bytes", bytes),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.2} {}B", n, prefix),
    }
}

pub fn pretty_print_percent(numerator: u64, denominator: u64) -> String
{
    format!("{:7.3}%", numerator as f64 / denominator as f64 * 100.0)
}

//TODO: I don't know how to convert u64 into f64 and allow precision loss generically
//pub fn pretty_print_bytes<T>(value : T) -> String
//{
//    match decimal_prefix(value) {
//        Standalone(bytes)   => format!("{} bytes", bytes),
//        Prefixed(prefix, n) => format!("{:.0} {}B", n, prefix),
//    }
//}
//pub fn pretty_print_percent<T,V>(numerator: T, denominator: V) -> String
//where T : Into<f64>,
//V : Into<f64>
//{
//    let numerator_as_float = numerator.into();
//    let denominator_as_float = denominator.into();
//    format!("{}%", numerator_as_float / denominator_as_float * 100.0)
//}


pub struct FileTypeIterator<'s> {
    walkdir_iterator : walkdir::IntoIter,
    file_type : &'s str,
}

impl<'s> FileTypeIterator<'s> {
    pub fn new(root : &'s str, file_type : &'s str) -> FileTypeIterator<'s>
    {
        FileTypeIterator {
            walkdir_iterator : walkdir::WalkDir::new(root).into_iter(),
            file_type : file_type,
        }
    }
}

impl<'s> Iterator for FileTypeIterator<'s>  {
type Item = walkdir::DirEntry;
	fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            let entry = match self.walkdir_iterator.next() {
                None => return None,
                Some(ent) => ent.unwrap(),
            };

            let is_file = entry.file_type().is_file();

            let extension_matches = match entry.path().extension() {
                None => false,
                Some(ext) => ext == self.file_type,
            };

            if is_file && extension_matches {
                return Some(entry);
            }
            else
            {
                continue
            }
        }
	}
}

pub enum VerificationResult {
    ExactMatch,     //images match exactly
    InvisibleMatch, //images match, except for pixels whose alpha values are 0
    Failure,         //images do not match
    NotFound,
}

pub fn verify_images(input_folder : &str, output_folder : &str) -> VerificationResult
{
    //iterate over each image in input folder
    let mut invisible_error = false;
    for (img_count, ent) in FileTypeIterator::new(input_folder, "png").enumerate()
    {
        //load input and output images
        let input_image_raw = image::open(ent.path()).unwrap().to_rgba8().into_raw();

        let path_relative_to_input_folder = ent.path().strip_prefix(input_folder).unwrap();
        let output_folder_image_path = Path::new(output_folder).join(path_relative_to_input_folder);

        match image::open(&output_folder_image_path) {
            Ok(output_image) =>  {
                let output_image_raw = output_image.to_rgba8().into_raw();

                println!("Comparing '{}' against '{}'...", ent.path().to_str().unwrap(), output_folder_image_path.to_str().unwrap());

                //TODO: don't use this silly array indexing...
                let mut invisible_pixel_found = false;
                for pixel_i in 0..input_image_raw.len()/4
                {
                    let i = pixel_i * 4;
                    let input_pixel = &input_image_raw[i..i + 4];
                    let output_pixel = &output_image_raw[i..i + 4];

                    if input_pixel != output_pixel {
                        //if both pixel's alpha values are 0, mark as invisible pixel
                        if (input_pixel[3]) == 0 && (output_pixel[3] == 0) {
                            invisible_pixel_found = true;
                        }
                        else
                        {
                            //found a really wrong pixel, just exit immediately
                            println!("Error: image {} does not match (true error)!", ent.path().to_str().unwrap());
                            println!("{:?} != {:?}", input_pixel, output_pixel);
                            return VerificationResult::Failure;
                        }
                    }
                }

                if invisible_pixel_found {
                    println!("WARNING: invisible pixel found");
                    invisible_error = true;
                }
             }

            Err(e) => {
                println!("{}", e.to_string());
                return VerificationResult::NotFound;
            }
        }
    }

    if invisible_error {
        return VerificationResult::InvisibleMatch;
    }

    return VerificationResult::ExactMatch;
}

pub fn get_byte_of_u64(value : u64, which_byte : usize) -> u8
{
    return (value >> (which_byte * 8)) as u8;
}

pub fn get_u64_mask_of_byte(value: u8, which_byte : usize) -> u64
{
    return (value as u64) << (which_byte * 8);
}

pub fn u64_to_u8_buf_little_endian(value : u64) -> [u8; 8]
{
    let mut buf = [0; 8];
    for i in 0..8
    {
        buf[i] = get_byte_of_u64(value, i);
    }
    buf
}

//convert 8 bytes from a 8 byte array into a u32 value, little endian
pub fn u8_buf_to_u64_little_endian(buf : &[u8; 8]) -> u64
{
    let mut returned_value = 0;
    for i in 0..8
    {
        returned_value |= get_u64_mask_of_byte(buf[i], i);
    }
    returned_value
}

//convert 4 bytes from a 4 byte array into a u32 value, big endian
pub fn u8_buf_to_u32_big_endian(buf : &[u8; 4]) -> u32
{
    (buf[0] as u32) << 24 |
    (buf[1] as u32) << 16 |
    (buf[2] as u32) << 8  |
    (buf[3] as u32)
}

//convert 4 bytes from a stream into a u32 value, big endian
pub fn u8_stream_to_u32_big_endian(reader : &mut dyn Read) -> u32
{
    let mut png_width_bytes  = [0u8; 4];
    reader.read_exact(&mut png_width_bytes).unwrap();
    return u8_buf_to_u32_big_endian(&png_width_bytes);
}

/// Read the width and height of a .png file
pub fn get_png_dimensions(reader : &mut dyn Read) -> Result<(u32, u32), &'static str>
{
    let reference_png_header : [u8; 16] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, //PNG header (always the same)
         0,   0,    0,    13, //this is the first chunk's length variable
        73,   72,   68,   82, //'IHDR' header
    ];

    let mut actual_png_header = [0u8; 16];
    reader.read_exact(&mut actual_png_header).unwrap(); //read the first 16 bytes

    if reference_png_header != actual_png_header {
        return Err("Incorrect PNG header");
    }

    //the chunk data of the IHDR chunk
    //stored as big endian
    let width = u8_stream_to_u32_big_endian(reader);  //read the next 4 bytes (the width)
    let height = u8_stream_to_u32_big_endian(reader); //read the next 4 bytes (the height)

    Ok((width, height))
}

//TODO: handle the case where images width/height is all 0?
pub fn scan_folder_for_max_png_size(input_folder : &str) -> (u32, u32)
{
    let mut max_width = 0;
    let mut max_height = 0;
    //iterate over each image in input folder
    for entry in walkdir::WalkDir::new(input_folder)
    {
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        let file_path = ent.path().to_str().unwrap();
        let png_file = File::open(file_path).unwrap();
        let mut reader = BufReader::new(png_file);
        let (width, height) = get_png_dimensions(&mut reader).expect("Could not read png file dimensions!");

        max_width = std::cmp::max(max_width, width);
        max_height = std::cmp::max(max_height, height);

        println!("Image {} width {} height {}", file_path, width, height);
    }

    (max_width, max_height)
}

pub fn save_image_no_alpha(mut image : RgbaImage, save_path : &str)
{
    println!("WARNING: Saving {} in Debug Mode: Alpha channel ignored!", save_path);
    for pixel in image.pixels_mut()
    {
        *pixel = image::Rgba([
            pixel[0],
            pixel[1],
            pixel[2],
            255
        ]);
    }

    let save_path = Path::new("debug_images").join(save_path);
    fs::create_dir_all(save_path.parent().unwrap()).unwrap();
    println!("Will save image to: {}", save_path.to_str().unwrap());
    image.save(save_path).unwrap()
}

pub fn compress_image_to_buffer(img: &image::RgbaImage) -> Vec<u8>
{
    let mut retvec = Vec::with_capacity(10000);
    let imgclone = img.clone();
    {
        let mut compressor = brotli::CompressorWriter::new(
            &mut retvec,
            BROTLI_BUFFER_SIZE,
            11,
            24);

        compressor.write(&imgclone.into_vec()).unwrap();
    }

    return retvec;
}


pub fn compress_buffer(img: &Vec<u8>) -> Vec<u8>
{
    let mut retvec = Vec::with_capacity(10000);
    let imgclone = img.clone();
    {
        let mut compressor = brotli::CompressorWriter::new(
            &mut retvec,
            BROTLI_BUFFER_SIZE,
            11,
            24);

        compressor.write(&imgclone).unwrap();
    }

    return retvec;
}


pub struct BlockXYIterator {
    block_size : usize,
    dimensions : (usize, usize),
    num_x_blocks : usize,
    num_y_blocks : usize,
    i : usize,
}

impl BlockXYIterator {
    pub fn new(block_size : usize, dimensions : (usize, usize)) -> BlockXYIterator
    {
        let mut safe_dimensions = dimensions;
        let mut safe_i = 0;

        // if dimensions given are 0,0, force dimensions to 1,1, then set i so that iterator
        // will terminate immediately. This avoids a modulo (%) by zero error and does 0 iterations.
        if dimensions.0 == 0 || dimensions.1 == 0 {
            safe_dimensions = (1, 1);
            safe_i = 2;
        }

        let num_x_blocks = safe_dimensions.0 / block_size + if safe_dimensions.0 % block_size == 0 {0} else {1};
        let num_y_blocks = safe_dimensions.1 / block_size + if safe_dimensions.1 % block_size == 0 {0} else {1};

        BlockXYIterator {
            block_size,
            dimensions : safe_dimensions,
            num_x_blocks,
            num_y_blocks,
            i : safe_i,
        }
    }
}

impl Iterator for BlockXYIterator {
type Item = (u32, u32);

//	//use this one?

    fn next(&mut self) -> Option<Self::Item>
    {
        if true {
            loop {
                let x_in_block = self.i % self.block_size;
                let y_in_block = (self.i / self.block_size) % self.block_size;

                let x_block = (self.i / (self.block_size * self.block_size)) % self.num_x_blocks;
                let y_block = self.i / (self.block_size * self.block_size * self.num_x_blocks);

                //println!("x_block: {} y_block: {}", x_block, y_block);

                let returned_x = (x_in_block + x_block * self.block_size) as u32;
                let returned_y = (y_in_block + y_block * self.block_size) as u32;

                self.i += 1;

                let x_in_range = returned_x < (self.dimensions.0 as u32);
                let y_in_range = returned_y < (self.dimensions.1 as u32);

                if x_in_range && y_in_range {
                    return Some((returned_x, returned_y));
                } else if y_block >= self.num_y_blocks {
                    return None;
                }
            }
        } else {

            let x = (self.i % self.dimensions.0) as u32;
            let y = (self.i / self.dimensions.0) as u32;

            self.i += 1;

            if y < self.dimensions.1 as u32 {
                Some((x, y))
            }
            else {
                None
            }
        }
    }
}




/*
Tests for xy iterator

//test iterating (50, width = 0, height = 0)
//test odd/even pixel width
//test images equal exactly the block width, and not equal to block width

#[allow(unused_imports)]
use common::BlockXYIterator;
#[allow(unused_imports)]
use image::RgbaImage;


  test for iterator

  let im = RgbaImage::new(3,5);
    let mut count = 0;
   for (x,y) in BlockXYIterator::new(2, (3, 5)) {
       count += 1;
       let pix = im.get_pixel(x,y);
       println!("x: {} y:{} {:?} {}", x,y, pix, count);
    }


    return;
*/


pub fn try_get_pixel(prev_xy : (i64, i64), prev_image : &image::RgbaImage) -> Option<image::Rgba<u8>>
{
    let prev_x = prev_xy.0; //original_pixel_xy.0 + prev_x_offset;
    let prev_y = prev_xy.1; //original_pixel_xy.1 + prev_y_offset;

    if prev_x < 0 || prev_y < 0 || prev_x >= prev_image.width() as i64 || prev_y >= prev_image.height() as i64 {
        return None;
    }

    return Some(*prev_image.get_pixel(prev_x as u32, prev_y as u32));
}