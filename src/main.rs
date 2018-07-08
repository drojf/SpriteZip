///
///Note: please run with --release (or higher optimization level), otherwise running program is way too slow.
///TODO: currently if there is a non-.png file in the input folder, the program will panic
///      should make an iterator which only processes .png files!
///

//rust file modules
mod alphablend;
mod common;
mod compress;
mod extract;

//crates
#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

//external crates
extern crate image;
extern crate brotli;
extern crate walkdir;

//standard crates
extern crate core;
extern crate time;

//custom modules
use alphablend::convert_folder_to_alphablend;
use compress::compress_path;
use extract::extract_archive;
use common::verify_images;

//standard uses
use std::path::{Path};

fn main()
{
    //create input images folder if it doesn't already exist:
    let input_path = Path::new("input_images");
    std::fs::create_dir_all(input_path).unwrap();

    let do_brotli_compression = true;
    let do_brotli_extract = true;
    let do_brotli_verify = true;

    let do_onscripter_alphablend = false;

    let output_basename = "compressed_images";
    let brotli_archive_path = format!("{}.brotli", output_basename);

    if do_brotli_compression
    {
        println!("\n\n ---------- Begin Compression... ---------- ");
        compress_path(&brotli_archive_path, false);
    }

    if do_brotli_extract
    {
        println!("\n\n ---------- Begin Extraction... ---------- ");
        extract_archive(&brotli_archive_path, false);
    }

    if do_brotli_verify
    {
        println!("\n\n ---------- Begin Verification... ---------- ");
        println!("Verification Result: {}",
            if verify_images("input_images", "output_images") {"SUCCESS"} else {"FAILURE"}
        );
    }

    if do_onscripter_alphablend
    {
        let num_converted = convert_folder_to_alphablend();

        if num_converted == 0
        {
            println!("Please place .png files/folders in the 'input_images' directory. They will be converted and placed in the 'output_images' directory.");
            return;
        }

    }
}
