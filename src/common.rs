use image;
use walkdir::WalkDir;
use std::path::Path;

pub struct Rectangle  {
    pub width: u32,
    pub height: u32,
    pub brotli_buffer_size: usize,
    pub brotli_quality: u32,
    pub brotli_window: u32,
}

pub static CANVAS_SETTING : Rectangle = Rectangle {
    width : 3000,
    height: 3000,
    brotli_buffer_size: 4096,
    brotli_quality: 9, //11, //9 seems to be a good tradeoff...changing q doesn't seem to make much diff though?
    brotli_window: 22
};

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