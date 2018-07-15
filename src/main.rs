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
extern crate oxipng;
extern crate png;

//standard crates
extern crate core;
extern crate time;

//custom modules
use alphablend::convert_folder_to_alphablend;
use compress::alt_compression_2;
use extract::extract_archive_alt;
use common::verify_images;
use common::VerificationResult;

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
fn do_extraction(brotli_archive_path : &str, oxipng_options : Option<oxipng::Options>, debug_mode : bool)
{
    println!("\n\n ---------- Begin Extraction... ---------- ");
    if !Path::new(brotli_archive_path).exists() {
        println!("ERROR: Archive file [{}] does not exist! exiting...", brotli_archive_path);
        std::process::exit(-1);
    }
    extract_archive_alt(&brotli_archive_path, oxipng_options, debug_mode);
}

fn do_verify(input_folder: &str, output_folder: &str)
{
    println!("\n\n ---------- Begin Verification... ---------- ");
    println!("Verification Result:");

    match verify_images(input_folder, output_folder) {
        VerificationResult::ExactMatch => println!("All images match exactly!"),
        VerificationResult::InvisibleMatch => println!("Warning - some pixels had invisible pixels with different values. They might have been optimized away by oxipng!"),
        VerificationResult::Failure => println!("Error: at least one image did not match!"),
        VerificationResult::NotFound => println!("Error: corresponding output image can't be opened or doesn't exist!"),
    }
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

fn print_description_and_exit() -> !
{
    println!("\n------------------------------- Usage Instructions -------------------------------------");
    println!("spritezip [compress|extract [0|1|2|3|4|5|6]|verify|selftest|alphablend]");
    println!("If you use 'spritezip extract' by itself, .png files are not optimized");
    println!("Specifying a number (2 is recommended) will cause oxipng to optimize the .png files before saving them.");
    println!("For example 'spritezip extract 2' will use level 2 compression (where 0 is fast and largest size, 6 is extremely slow and smallest size)");
    std::process::exit(-1);
}

fn main()
{
    let input_folder = "input_images";
    let output_folder = "output_images";
    let brotli_archive_path = "compressed_images.brotli";

    println!("Spritezip version 0.1.3\n");

    //create input images folder if it doesn't already exist:
    let input_path = Path::new(input_folder);
    std::fs::create_dir_all(input_path).unwrap();

    //check if the output folder already exists
    let output_folder_exists = Path::new(output_folder).exists();

    //TODO: use 'clap' to parse arugments
    //Use command line arguments to set program mode
    let args: Vec<String> = env::args().collect();

    //set debug mode if 'debug' in argument list
    let mut debug_mode = false;
    for s in &args {
        if s == "debug" {
            debug_mode = true;
        }
    }

    let mode = if args.len() < 2 {
        None
    } else {
        Some(args[1].as_ref())
    };

    //get oxipng optimization level
    let oxipng_options = if mode != Some("extract") {
            None
        }
        else if args.len() < 3 {
            println!("INFO: 'optimize' argument NOT given - PNG files will not be optimized for size when extracting!");
            None
        } else {
            match args[2].parse::<u8>() {
                Ok(optimization_level) => {
                    println!("INFO: optimize level [{}] given - PNG files will be optimized for size when extracting", optimization_level);
                    println!("INFO: Note: optimization levels are from 0 (fast, low comp) to 6 (slow, high comp). Level 2 is recommended. Values higher than 6 will be the same as level 6");
                    println!("See the oxipng documentation for more details.");
                    let mut oxipng_options = oxipng::Options::from_preset(optimization_level);
                    oxipng_options.verbosity = if debug_mode { Some(0) } else { None }; //supress oxipng printing unless in debug mode
                    oxipng_options.interlace = Some(0);                                 //remove any interlacing from image
                    //don't try to change bit depth/color type/palette in case it breaks the game where the sprite is used
                    oxipng_options.bit_depth_reduction = false;
                    oxipng_options.color_type_reduction = false;
                    oxipng_options.palette_reduction = false;
                    println!("Oxipng will use {} threads", oxipng_options.threads);
                    Some(oxipng_options)
                },
                Err(e) => {
                    //TODO: if you specify 'debug' with no oxipng level, it will show this error!
                    println!("ERROR: Invalid value for {} 'optimization' argument (reason: {}) - exiting", &args[2], e.to_string());
                    print_description_and_exit();
                },
            }
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
            do_extraction(brotli_archive_path, oxipng_options, debug_mode);
        },
        Some("verify") => {
            do_verify(input_folder, output_folder);
        }
        Some("selftest") => {
            if output_folder_exists {
                println!("ERROR: Can't run Self Test because output folder already exists!");
                println!("Please delete the folder [{}] as it may already contain 'correct' files, giving a false test result", output_folder);
                print_description_and_exit();
            }

            do_compression(brotli_archive_path);
            do_extraction(brotli_archive_path, oxipng_options, debug_mode);
            do_verify(input_folder, output_folder);
        },
        Some("alphablend") => {
            do_alphablend();
        },
        Some(_) => {
            print_description_and_exit();
        }
    }
}
