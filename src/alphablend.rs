//standard uses
use std::path::{Path};
use std::fs;

//nonstandard includes
use image;
use image::{RgbImage, RgbaImage};
use walkdir::WalkDir;

fn convert_alphablend_to_transparent_png(filepath : &str, save_path : &str)
{
    let img_dyn = image::open(filepath).unwrap();
    let img = img_dyn.to_rgba8(); //I'm not sure if you can use 'as_rgba8' for 'rgb' images, so just use 'to_rgba8'

    //create new image whose size is half the width of the original image, with a proper alpha channel
    let transparent_png_width = img.width() / 2;
    let mut transparent_png = RgbaImage::new(transparent_png_width, img.height());

    for (x, y, pixel) in transparent_png.enumerate_pixels_mut()
    {
        let color_pixel = img.get_pixel(x,y);
        let alpha_pixel = img.get_pixel(x + transparent_png_width, y);

        *pixel = image::Rgba( [color_pixel[0], color_pixel[1], color_pixel[2], 0xFF - alpha_pixel[0]] );
    }

    transparent_png.save(save_path).unwrap();
}

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
pub fn convert_folder_to_alphablend(reverse : bool) -> u32
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

        //force .png extension, then convert to string
        let save_path_as_png = output_path.with_extension("png");
        let save_path = save_path_as_png.to_str().unwrap();

        //let file_name_no_ext = ent.path().file_stem().unwrap().to_str().unwrap();
        //let save_path = [file_name_no_ext, ".png"].concat();

        println!("Will save image to: {}", save_path);

        //create the save directory if it doesn't already exist:
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();

        if reverse{
            convert_alphablend_to_transparent_png(ent.path().to_str().unwrap(), &save_path);
        }
        else {
            convert_to_onscripter_alphablend(ent.path().to_str().unwrap(), &save_path);
        }

        count += 1;
    }

    count
}