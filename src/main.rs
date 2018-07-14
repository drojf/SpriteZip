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
extern crate bincode;

//external crates
extern crate image;
extern crate brotli;
extern crate walkdir;
extern crate number_prefix;

//standard crates
extern crate core;
extern crate time;

//custom modules
use alphablend::convert_folder_to_alphablend;
use compress::alt_compression_2;
use extract::extract_archive_alt;
use common::verify_images;

//standard uses
use std::path::{Path};
use std::env;

//TODO: take input/output folders as arguments
fn do_compression(brotli_archive_path : &str)
{
    println!("\n\n ---------- Begin Compression... ---------- ");
    alt_compression_2(&brotli_archive_path);
}

//TODO: take input/output folders as arguments
fn do_extraction(brotli_archive_path : &str)
{
    println!("\n\n ---------- Begin Extraction... ---------- ");
    if !Path::new(brotli_archive_path).exists() {
        println!("ERROR: Archive file [{}] does not exist! exiting...", brotli_archive_path);
        std::process::exit(-1);
    }
    extract_archive_alt(&brotli_archive_path, false);
}

fn do_verify(input_folder: &str, output_folder: &str)
{
    println!("\n\n ---------- Begin Verification... ---------- ");
    println!("Verification Result: {}",
        if verify_images(input_folder, output_folder) {"SUCCESS"} else {"FAILURE"}
    );
}

fn do_alphablend()
{
    let num_converted = convert_folder_to_alphablend();
    if num_converted == 0
    {
        println!("Please place .png files/folders in the 'input_images' directory. They will be converted and placed in the 'output_images' directory.");
        return;
    }
}

fn main()
{
    let input_folder = "input_images";
    let output_folder = "output_images";
    let brotli_archive_path = "compressed_images.brotli";

    println!("Spritezip version 0.1\n");

    //create input images folder if it doesn't already exist:
    let input_path = Path::new(input_folder);
    std::fs::create_dir_all(input_path).unwrap();

    //check if the output folder already exists
    let output_folder_exists = Path::new(output_folder).exists();

    //Use command line arguments to set program mode
    let args: Vec<String> = env::args().collect();
    let mode = if args.len() < 2 {
        None
    } else {
        Some(args[1].as_ref())
    };

    match mode {
        Some("compress") => {
            //TODO: compression produces an output file, even if input images directory is empty
            do_compression(brotli_archive_path);
        },
        Some("extract") | None => {
            if mode == None {
                println!("No arguments supplied - will try to extract the default archive [{}]...", brotli_archive_path);
            }
            do_extraction(brotli_archive_path);
        },
        Some("selftest") => {
            if output_folder_exists {
                println!("ERROR: Can't run Self Test because output folder already exists!");
                println!("Please delete the folder [{}] as it may already contain 'correct' files, giving a false test result", output_folder);
                std::process::exit(-1);
            }

            do_compression(brotli_archive_path);
            do_extraction(brotli_archive_path);
            do_verify(input_folder, output_folder);
        },
        Some("alphablend") => {
            do_alphablend();
        },
        Some(_) => {
            println!("Invalid argument Please run as 'spritezip [compress|extract|verify|selftest|alphablend]'");
        }
    }
}
