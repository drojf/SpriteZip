extern crate image;
extern crate walkdir;

//standard uses
use std::path::{Path};
use std::io::{Write};
use std::io;
use std::fs;
use std::fs::File;

use image::{RgbImage, RgbaImage, GenericImage};

use walkdir::WalkDir;

fn convert_to_onscripter_alphablend(filepath : &str, save_path : &str)
{
    let img_dyn = image::open(filepath).unwrap();
    let img = img_dyn.as_rgba8().unwrap();

    //create new image whose size is twice the width of the original image, with no alpha channel
    let mut rgb_left_alpha_right = RgbImage::new(img.width() * 2, img.height());

    //save color image to lhs
    for (x, y, pixel) in img.enumerate_pixels()
    {
        rgb_left_alpha_right.put_pixel(x, y, image::Rgb([pixel[0], pixel[1], pixel[2]]));
    }

    //save alpha image to rhs (NOTE: is inverted from normal - black indicates transparent, white indicates solid)
    for (x, y, pixel) in img.enumerate_pixels()
    {
        let value = 0xFF - pixel[3];
        rgb_left_alpha_right.put_pixel(x+img.width(), y, image::Rgb([value, value, value]));
    }

    //save new_image
    rgb_left_alpha_right.save(save_path).unwrap();
}

//TODO: output to 'output_dir' instead of same directory.
pub fn convert_folder_to_alphablend() -> u32
{
    let mut count = 0;
    let recursive_path_iter = WalkDir::new("input_images");
    for entry in recursive_path_iter
    {
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        println!("\nProcessing Image: '{}'", ent.path().display());

        let path_with_input_images_as_root = ent.path().strip_prefix("input_images").unwrap();

        println!("path with input images as root: {}", path_with_input_images_as_root.to_str().unwrap());

        let output_path = Path::new("output_images").join(path_with_input_images_as_root);
        println!("output path: {}", output_path.to_str().unwrap());

        let save_path = output_path.to_str().unwrap();
        //let file_name_no_ext = ent.path().file_stem().unwrap().to_str().unwrap();
        //let save_path = [file_name_no_ext, ".png"].concat();

        println!("Will save image to: {}", save_path);

        //create the save directory if it doesn't already exist:
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();

        convert_to_onscripter_alphablend(ent.path().to_str().unwrap(), &save_path);

        count += 1;
    }

    count
}