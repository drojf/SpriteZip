///
///Note: please run with --release (or higher optimization level), otherwise running program is way too slow.
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

//standard uses
use std::path::{Path};
use std::io;
use std::fs;

fn pause()
{
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

fn main()
{
    //create input images folder if it doesn't already exist:
    let input_path = Path::new("input_images");
    std::fs::create_dir_all(input_path).unwrap();

    let do_brotli_compression = false;
    let do_brotli_extract = true;
    let do_onscripter_alphablend = false;

    let output_basename = "compressed_images";
    let brotli_archive_path = format!("{}.brotli", output_basename);
    let metadata_path = format!("{}.metadata", output_basename);

    if do_brotli_compression
    {
        compress_path(&brotli_archive_path, &metadata_path);
    }
    else if do_brotli_extract
    {
        extract_archive(&brotli_archive_path, &metadata_path);
    }
    else if do_onscripter_alphablend
    {
        let num_converted = convert_folder_to_alphablend();

        if num_converted == 0
        {
            println!("Please place .png files/folders in the 'input_images' directory. They will be converted and placed in the 'output_images' directory.");
            println!("Press any key to continue...");
            pause();
            return;
        }

    }
    
    println!("All done. Press enter to continue...");
    pause();
}
