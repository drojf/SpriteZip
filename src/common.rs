use image;
use walkdir::WalkDir;
use std;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

pub const FILE_FORMAT_HEADER_LENGTH: usize = 8;

pub struct Rectangle  {
    pub brotli_buffer_size: usize,
    pub brotli_quality: u32,
    pub brotli_window: u32,
}

pub static CANVAS_SETTING : Rectangle = Rectangle {
    brotli_buffer_size: 4096,
    brotli_quality: 9, //11, //9 seems to be a good tradeoff...changing q doesn't seem to make much diff though?
    brotli_window: 22,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct DecompressionInfo {
    pub canvas_size: (u32, u32),
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


// TODO: crop diff'd images  so that not so much data needs to be compressed?
/// Subtracts the canvas image from the given image, where the given image is assumed to be smaller
/// than the canvas
/// Eg: Performs [image - canvas] for all pixels in image.
/// x_offset, y_offset: offsets image before performing the subtraction
pub fn subtract_image_from_canvas(canvas: &mut image::RgbaImage, img : &image::RgbaImage, x_offset : u32, y_offset : u32)
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


// TODO: crop diff'd images  so that not so much data needs to be compressed?
/// Subtracts the canvas image from the given image, where the given image is assumed to be smaller
/// than the canvas
/// Eg: Performs [image - canvas] for all pixels in image.
/// x_offset, y_offset: offsets image before performing the subtraction
pub fn add_image_to_canvas(canvas: &mut image::RgbaImage, img : &image::RgbaImage, x_offset : u32, y_offset : u32)
{
    for (x, y, pixel) in img.enumerate_pixels()
    {
        let mut canvas_pixel = canvas.get_pixel_mut(x + x_offset, y + y_offset);

        //TODO: disable debug mode to use alpha value
        //must specify u8 to ensure wrapping occurs
        let new_pixel : [u8; 4] = [
            pixel[0].wrapping_add(canvas_pixel[0]),
            pixel[1].wrapping_add(canvas_pixel[1]),
            pixel[2].wrapping_add(canvas_pixel[2]),
            pixel[3].wrapping_add(canvas_pixel[3]),
        ];

        *canvas_pixel = image::Rgba(new_pixel);
    }
}

pub fn offset_to_bottom_center_image_value(canvas_size : (u32, u32), img_size : (u32, u32)) -> (u32, u32)
{
    //Calculate image offset such that image is placed at the center bottom of the canvas.
    let x_offset = (canvas_size.0 - img_size.0) / 2;
    let y_offset = canvas_size.1 - img_size.1;
    (x_offset, y_offset)
}

pub fn offset_to_bottom_center_image(canvas: &image::RgbaImage, img : &image::RgbaImage) -> (u32, u32)
{
    offset_to_bottom_center_image_value((canvas.width(), canvas.height()), (img.width(), img.height()))
}

pub fn verify_images(input_folder : &str, output_folder : &str) -> bool
{
    //iterate over each image in input folder
    for entry in WalkDir::new(input_folder)
    {
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        //load input and output images
        let input_image_raw = image::open(ent.path()).unwrap().raw_pixels();

        let path_relative_to_input_folder = ent.path().strip_prefix(input_folder).unwrap();
        let output_folder_image_path = Path::new(output_folder).join(path_relative_to_input_folder);

        match image::open(&output_folder_image_path) {
            Ok(output_image) =>  {
                let output_image_raw = output_image.raw_pixels();

                println!("Comparing '{}' against '{}'...", ent.path().to_str().unwrap(), output_folder_image_path.to_str().unwrap());

                //compare the raw buffer representation of each and verify each byte matches
                for (input_b, output_b) in input_image_raw.iter().zip(output_image_raw.iter())
                {
                    if input_b != output_b {
                        println!("Error: image {} does not match!", ent.path().to_str().unwrap());
                        return false;
                    }
                }
            }

            Err(e) => {
                println!("Error: corresponding output image can't be opened or doesn't exist! {:?}", e);
                return false;
            }
        }
    }

    return true;
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
pub fn u8_stream_to_u32_big_endian(reader : &mut Read) -> u32
{
    let mut png_width_bytes  = [0u8; 4];
    reader.read_exact(&mut png_width_bytes).unwrap();
    return u8_buf_to_u32_big_endian(&png_width_bytes);
}

/// Read the width and height of a .png file
pub fn get_png_dimensions(reader : &mut Read) -> Result<(u32, u32), &'static str>
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
    for entry in WalkDir::new(input_folder)
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