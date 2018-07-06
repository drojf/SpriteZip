use std::fs;

pub fn extract_archive(brotli_archive_path : &str, metadata_path : &str) {
    let data = fs::read(metadata_path).expect("Unable to read metadata file");

    println!("Loaded metadata file: {}", data.len());

    //unserialize the metadata file

    //open the brotli file for reading

    //initialize the canvas

    //for each image
        //partially decompress the brotli file
        //add the diff to the canvas at the specified coordinates
        //get the correct crop of the canvas (using metadata) as a new image
        //save the reconstructed image as .png file
}