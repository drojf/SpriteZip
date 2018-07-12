//standard uses
use std;
use std::io::{Write};
use std::fs::File;
use time;
use std::io::SeekFrom;
use std::io::Seek;

//non-standard use
use serde_json;
use bincode;

use brotli;
use image;
use image::{RgbaImage, GenericImage};
use walkdir::WalkDir;

use common::{CompressedImageInfo, DecompressionInfo};
use common::subtract_image_from_canvas;
use common::offset_to_bottom_center_image;
use common::scan_folder_for_max_png_size;
use common::u64_to_u8_buf_little_endian;
use common::save_image_no_alpha;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::{convert_pixel_based_to_channel_based, compress_image_to_buffer, compress_buffer};

struct CroppedImageBounds {
    x : u32,
    y : u32,
    width : u32,
    height : u32,
}

//TODO: cropped image cannot be bigger than two input images - can optimize for this
/// This function takes in an image, and returns the bounding box of the image.
/// It assumes that pixels whose components are all 0 are not important. All other
/// pixels will be counted when calculating the bounding box
/// img: image to find the bounding box
/// x_offset, y_offset: within the image, the x and y offset to begin searching for the bounding box
/// max_width, max height: the max size to search for the bounding box within the image, starting from
/// the x and y offset
/// (the x and y offset and max width/height let you specify a subimage in which to find the bounding box of the image)
/// return value: the bounding box of the image (given as offset & size). Note that the returned
/// coordinates are in absolute coordinates, not relative to the x_offset and y_offset input values
/// returns number of pixels which are identical in both images
fn crop_function(img: &image::RgbaImage, offset : (u32, u32), max_width : u32, max_height : u32) -> (CroppedImageBounds, u64)
{
    //TODO: figure out a better rusty way to do this
    let mut x0 = offset.0 + (max_width-1);
    let mut x1 = offset.0;
    let mut y0 = offset.1 + (max_height-1);
    let mut y1 = offset.1;

    let mut num_identical_pixels = 0u64;
    for (x, y, pixel) in img.enumerate_pixels()
    {
        //todo: just make own for loop which just iterates over the required area instead of whole canvas
        if x < offset.0 || y < offset.1 ||
           x >= (offset.0 + max_width) || y >= (offset.1 + max_height)
        {
            continue
        }

        if *pixel != image::Rgba([0,0,0,0])
        {
            x0 = std::cmp::min(x, x0);
            y0 = std::cmp::min(y, y0);
            x1 = std::cmp::max(x, x1);
            y1 = std::cmp::max(y, y1);
        }
        else
        {
            num_identical_pixels += 1;
        }
    }

    (CroppedImageBounds {
        x: x0, y: y0,
        width: x1-x0+1, height: y1-y0+1,
    },
    num_identical_pixels)
}

struct Cropper {
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
}

#[derive(Debug)]
pub struct CropRegion {
    top_left : (u32, u32),
    dimensions : (u32, u32),
}

impl Cropper {

    fn new(image_dimensions : (u32, u32)) -> Cropper {
        Cropper {
            //set each bound to its worst case value
            min_x: image_dimensions.0,
            max_x: 0,
            min_y: image_dimensions.1,
            max_y: 0,
        }
    }

    fn add_nonzero_pixel(&mut self, x : u32, y : u32)
    {
        //reduce each bound
        self.min_x = std::cmp::min(self.min_x, x);
        self.max_x = std::cmp::max(self.max_x, x);

        self.min_y = std::cmp::min(self.min_y, y);
        self.max_y = std::cmp::max(self.max_y, y);
    }

    //not sure whether to return None or a zero size crop region here
    //I guess a zero size crop region is more generic, so I'll do that
    fn get_crop_region(&self) -> CropRegion
    {
        //if the min x is greater than the max x, it means the image is empty (no pixels ever recorded)
        if self.min_x > self.max_x {
            CropRegion {
                top_left : (0,0),
                dimensions : (0,0),
            }
        }
        else {
            CropRegion {
                top_left : (self.min_x, self.min_y),
                dimensions : ((self.max_x - self.min_x + 1),(self.max_y - self.min_y + 1))
            }
        }
    }
}

fn measure_similarity(img1: &image::RgbaImage, img2: &image::RgbaImage)
{
    //initialize canvas to place the two images on
    let max_width = std::cmp::max(img1.width(), img2.width());
    let max_height = std::cmp::max(img1.height(), img2.height());
    let mut canvas = RgbaImage::new(max_width, max_height);

    //place first image on the canvas
    let (img1_x_offset, img1_y_offset) = offset_to_bottom_center_image(&canvas, &img1);
    canvas.copy_from(img1, img1_x_offset, img1_y_offset);

    //subtract second image from the canvas
    let img2_offset_from_bottom_center = offset_to_bottom_center_image(&canvas, &img2);
    subtract_image_from_canvas(&mut canvas, &img2, img2_offset_from_bottom_center);

    //count number of non-identical pixels which have non-zero alpha values
    let mut num_identical_pixels = 0;
    let mut num_different_pixels = 0;
    for (x, y, pixel_ref) in canvas.enumerate_pixels()
    {
        let pixel = *pixel_ref;

        //skip alpha = 0 pixels
        if pixel[3] == 0 { continue; }

        if pixel == image::Rgba([0u8; 4])
        {
            num_identical_pixels += 1;
        }
        else
        {
            num_different_pixels += 1;
        }
    }

    println!("Identical Pixels {} Different Pixels {}", num_identical_pixels, num_different_pixels);


    /*
    //calculate how much to offset such that is centered on canvas
    let img1_x_offset = max_width - img1.width()/2;
    let img2_x_offset = max_width - img2.width()/2;
    let

    //calculate how much to offset so that image is placed at bottom of image
    let img1_y_offset = max_height - img1.height();
    let img2_y_offset = max_height - img2.height();*/

}


struct BlockImageIterator<'s> {
    original_image : &'s image::RgbaImage,
    block_size : usize,
    i : usize,
}

impl<'s> BlockImageIterator<'s> {
    fn new(original_image : &'s image::RgbaImage, block_size : usize) -> BlockImageIterator<'s>
    {
        //let block_size = 50;
        //let num_x_blocks = original_image.width()/block_size  + if original_image.width()  % block_size == 0 { 0 } else { 1 };
        //let num_y_blocks = original_image.height()/block_size + if original_image.height() % block_size == 0 { 0 } else { 1 };

        BlockImageIterator {
            original_image,
            block_size,
            i : 0,
        }
    }
}

impl<'s> Iterator for BlockImageIterator<'s>  {
type Item = (u32, u32, image::Rgba < u8 >);

	//use this one?
	fn next(&mut self) -> Option<Self::Item>
    {
        let debug = false;

		let B = self.block_size;
        let i = self.i;
        let width = self.original_image.width() as usize;
        let height = self.original_image.height() as usize;
		let pixels_per_block_row = B * width;

        if debug { println!("i:{}", i); }

		let block_y = i / pixels_per_block_row;
		let pixels_in_previous_block_rows = block_y * pixels_per_block_row;
		let block_height =  std::cmp::min(B, height - B * block_y);
        if debug  { println!("block_y {} block_height {}", block_y, block_height); }

		//for all rows except the last row, block_height == B.
		//for last row, block_height = image.height() % B
		let pixels_in_current_block_row = i - pixels_in_previous_block_rows;
		let block_x = pixels_in_current_block_row / (B * block_height);
        let block_width  = std::cmp::min(B, width - B * block_x);
        if debug { println!("pixels_in_block_row  {} block_x {} block_width {}", pixels_in_current_block_row , block_x, block_width); }

		//for the very last block, both block height and block width will != B
		let i_in_block = pixels_in_current_block_row - block_x * (B * block_height);
        if debug { println!("i_in_block {}", i_in_block); }

		let x = (i_in_block % block_width + block_x * B) as u32;
        let y = (i_in_block / block_width + block_y * B) as u32;

        self.i += 1;

        if debug {
            println!("({:02},{:02})", x, y);
            println!("");
        }

        if y < height as u32 {
            Some((x, y, *self.original_image.get_pixel(x, y)))
        }
        else {
            None
        }
	}

}

pub fn get_offset_to_other_image(original_image : &image::RgbaImage, prev_image : &image::RgbaImage) -> (i64, i64)
{
    let prev_x_offset = (prev_image.width() as i64 - original_image.width()  as i64)/2;
    let prev_y_offset = prev_image.height() as i64 - original_image.height() as i64;
    (prev_x_offset, prev_y_offset)
}

pub fn try_get_pixel(prev_xy : (i64, i64), prev_image : &image::RgbaImage) -> Option<image::Rgba<u8>>
{
    let prev_x = prev_xy.0; //original_pixel_xy.0 + prev_x_offset;
    let prev_y = prev_xy.1; //original_pixel_xy.1 + prev_y_offset;

    if prev_x < 0 || prev_y < 0 || prev_x >= prev_image.width() as i64 || prev_y >= prev_image.height() as i64 {
        return None;
    }

    return Some(*prev_image.get_pixel(prev_x as u32, prev_y as u32));
}

pub fn alt_compression_2(brotli_archive_path : &str)
{
    let mut base_images : Vec<RgbaImage> = Vec::new();
    let mut relative_paths : Vec<String> = Vec::new();

    //Create compressors for image data and bitmap
    let brotli_quality = 11;
    let brotli_window = 24;
    let brotli_file = File::create("alt_image.brotli").expect("Cannot create file");
    let mut image_compressor = brotli::CompressorWriter::new(brotli_file,BROTLI_BUFFER_SIZE,brotli_quality,brotli_window);
    let brotli_file_2 = File::create("alt_diff.brotli").expect("Cannot create file");
    let mut bitmap_compressor = brotli::CompressorWriter::new(brotli_file_2, BROTLI_BUFFER_SIZE, brotli_quality, brotli_window);

    for entry in WalkDir::new("input_images")
    {
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();
        relative_paths.push(String::from(path_relative_to_input_folder));

        let img_dyn = image::open(ent.path()).unwrap();
        let img = img_dyn.as_rgba8().unwrap();
        base_images.push(img.clone());

        println!("Image Path: {}", path_relative_to_input_folder);
    }

    for i in 0..base_images.len() - 1 {
        let img1 = &base_images[i];
        let img2 = &base_images[i + 1];

        alt_compression_3_inner(img1, img2, &mut image_compressor, &mut bitmap_compressor);
    }

}

pub fn alt_compression_3_inner<'s,T>(original_image : &image::RgbaImage, prev_image : &image::RgbaImage, image_compressor : &'s mut brotli::CompressorWriter<T>, bitmap_compressor : &'s mut   brotli::CompressorWriter<T>) -> CropRegion
where T: std::io::Write
{
    let mut cropper = Cropper::new((original_image.width(), original_image.height()));

    let (x_offset_to_other_image, y_offset_to_other_image) = get_offset_to_other_image(original_image, prev_image);

    let mut difference : Vec<u8> = Vec::with_capacity(original_image.width() as usize * original_image.height() as usize);

    for (x,y,original_image_pixel) in BlockImageIterator::new(&original_image, 50)
    {
        let original_image_pixel = *original_image.get_pixel(x, y);
        let prev_x = (x as i64 + x_offset_to_other_image);
        let prev_y = (y as i64 + y_offset_to_other_image);

        let pixels_equal = match try_get_pixel((prev_x, prev_y), &prev_image) {
            None => false,
            Some(prev_pixel) => original_image_pixel == prev_pixel,
        };

        if pixels_equal {
            difference.push(0u8);
        }
        else {
            difference.push(1u8);
            image_compressor.write(&original_image_pixel.data);
            cropper.add_nonzero_pixel(x,y);
        }
    }

    //crop the difference map
    let crop_region = cropper.get_crop_region();
    println!("Crop region is {:?}", crop_region);
    let mut difference_cropped = Vec::with_capacity(crop_region.dimensions.0 as usize * crop_region.dimensions.1 as usize);
    for y in 0..crop_region.dimensions.1 as usize {
        for x in 0..crop_region.dimensions.0 as usize {
            difference_cropped.push(difference[x + y * original_image.width() as usize]);
        }
    }

    bitmap_compressor.write(&difference_cropped);

    //return crop_region to be saved as metadata
    return crop_region
}

pub fn alt_compression_2_inner<'s,T>(original_image : &image::RgbaImage, prev_image : &image::RgbaImage, image_compressor : &'s mut brotli::CompressorWriter<T>, bitmap_compressor : &'s mut   brotli::CompressorWriter<T>)
where T: std::io::Write
{
    let (x_offset_to_other_image, y_offset_to_other_image) = get_offset_to_other_image(original_image, prev_image);

    //TODO: crop difference image
    //let cropper = Cropper::new();

    let b = 50;
    let image_width = original_image.width();
    let image_height = original_image.height();

    let x_num_blocks = original_image.width()  / b + if original_image.width()  % b != 0 {1} else {0};
    let y_num_blocks = original_image.height() / b + if original_image.height() % b != 0 {1} else {0};

    let mut difference : Vec<u8> = Vec::with_capacity(original_image.width() as usize * original_image.height() as usize);

    for y_block_i in 0..y_num_blocks {
        for x_block_i in 0..x_num_blocks {

            let block_height = std::cmp::min(b, original_image.height() - b * y_block_i);
            for block_y_pixel in 0..block_height{

                let block_width = std::cmp::min(b, original_image.width() - b * x_block_i);
                for block_x_pixel in 0..block_width {

                    let x = block_x_pixel + x_block_i * b;
                    let y = block_y_pixel + y_block_i * b;
                    let original_image_pixel = *original_image.get_pixel(x, y);
                    let prev_x = (x as i64 + x_offset_to_other_image);
                    let prev_y = (y as i64 + y_offset_to_other_image);

                    //check if out of range of other image. If not out of range, check equality
                    //let pixels_equal = ;
                        //x_ref < ref_image_width  &&
                        //y_ref < ref_image_y  &&
                        //original_image_pixel == *prev_imag.get_pixel(x + ref_offset_x, y + ref_offset.y);;

                    let pixels_equal = match try_get_pixel((prev_x, prev_y), &prev_image) {
                        None => false,
                        Some(prev_pixel) => original_image_pixel == prev_pixel,
                    };

                    if pixels_equal {
                        difference.push(0u8);
                    }
                    else {
                        difference.push(1u8);
                        image_compressor.write(&original_image_pixel.data);
                        //TODO: update cropper
                        //cropper.update(x,y);
                    }
                }
            }
        }
    }

    //crop the difference map
    //TODO: crop difference map
    //difference_map = difference_map.crop(copper.values)
    bitmap_compressor.write(&difference);

    //TODO: return crop region
    //return crop_region //need to record metadata on which area of the image was cropped.


//iteration order should be blockwise, optionally pixels should be scanned in snake order/blocks
//also scanned in snake order.

//before first iteration,
// - init compressor object
// - set prev_image = black image the same size as first image

//for each image
    //init crop object with image width/height

    //for each x,y, pixel in image (iteration order should be optimized later)
        //pixels_are_identical ->
            //if x,y is out of range of prevImage ret NOT_EQUAL
            //ret [prev_image == image] (at the corresponding location, not just x,y)

        //if pixels are identical
            //difference_map[x,y] = 0
        //if pixels are not identical
            //difference_map[x,y] = 1
            //compressor.add(image[x,y])
            //update crop bounds

    //crop the difference map
    //compress the difference map

    //prev_image = image
}

#[derive(Debug)]
pub struct ImageOverlapInfo {
    pub overlap : (u32, u32),
    pub img1_offset : (u32, u32),
    pub img2_offset : (u32, u32),
}

fn calculate_overlapping_regions_when_bottom_centered(img1: &image::RgbaImage, img2: &image::RgbaImage) -> ImageOverlapInfo
{
    //calculate how much the two images overlap
    let x_overlap = std::cmp::min(img1.width(), img2.width());
    let y_overlap = std::cmp::min(img1.height(), img2.height());

    //start iterating so that you iterate over the center of each image
    let img1_x_start = (img1.width() - x_overlap)/2;
    let img2_x_start = (img2.width() - x_overlap)/2;

    //start iterating so that you take the bottom-most section of the image
    let img1_y_start = img1.height() - y_overlap;
    let img2_y_start = img2.height() - y_overlap;

    ImageOverlapInfo {
        overlap : (x_overlap, y_overlap),
        img1_offset : (img1_x_start, img1_y_start),
        img2_offset : (img2_x_start, img2_y_start),
    }
}

pub fn block_compression_test(brotli_archive_path : &str)
{
    let mut base_images : Vec<RgbaImage> = Vec::new();
    let mut relative_paths : Vec<String> = Vec::new();

    //add compressor here
    let brotli_quality = 11;
    let brotli_window = 24;
    let mut brotli_file = File::create("alt_image.brotli").expect("Cannot create file");
    let mut compressor = brotli::CompressorWriter::new(
    &brotli_file,
    BROTLI_BUFFER_SIZE,
    brotli_quality,
    brotli_window);

    for entry in WalkDir::new("input_images")
    {
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();
        relative_paths.push(String::from(path_relative_to_input_folder));

        let img_dyn = image::open(ent.path()).unwrap();
        let img = img_dyn.as_rgba8().unwrap();
        base_images.push(img.clone());

        println!("Image Path: {}", path_relative_to_input_folder);
    }

    for i in 0..base_images.len()-1 {
        let img1 = &base_images[i];
        let img2 = &base_images[i+1];

        let overlap_info = calculate_overlapping_regions_when_bottom_centered(img1, img2);
        println!("img1 is ({:?}), img2 is ({:?})", img1.dimensions(), img2.dimensions());
        println!("{:?}", overlap_info);
        let mut debug_image = RgbaImage::new(overlap_info.overlap.0, overlap_info.overlap.1);

        //do test on compression ratio


        //do a test on compression ratio of bitmapped image?
        let mut bitmap = vec![0u8;overlap_info.overlap.0 as usize * overlap_info.overlap.1 as usize];

        //for comparison, just compress overlapping region

        //iterate over overlapping section, giving count of different pixels in each grid square
        let block_size = 25;
        let x_num_blocks = overlap_info.overlap.0 / block_size + 1; //always do one extra, even if not needed (for now)
        let y_num_blocks = overlap_info.overlap.1 / block_size + 1; //always do one extra, even if not needed (for now)

        for y_block_i in 0..y_num_blocks {
            for x_block_i in 0..x_num_blocks {
                //iterate within each block
                let mut different_pixel_count = 0;
                for block_y_pixel in 0..block_size {
                    for block_x_pixel in 0..block_size {
                        let both_offset_x = x_block_i * block_size + block_x_pixel;
                        let both_offset_y = y_block_i * block_size + block_y_pixel;

                        if both_offset_x >= overlap_info.overlap.0 || both_offset_y >= overlap_info.overlap.1 {
                            continue
                        }

                        //compare pixels
                        let image_1_pixel = img1.get_pixel(both_offset_x + overlap_info.img1_offset.0, both_offset_y + overlap_info.img1_offset.1);
                        let image_2_pixel = img2.get_pixel(both_offset_x + overlap_info.img2_offset.0, both_offset_y + overlap_info.img2_offset.1);

                        if *image_1_pixel != *image_2_pixel {
                            different_pixel_count += 1;
                            debug_image.put_pixel(both_offset_x, both_offset_y,image::Rgba( [255,255,255, 255] ));
                            compressor.write(&image_2_pixel.data).unwrap();
                            bitmap[(both_offset_x + both_offset_y * overlap_info.overlap.0) as usize] = 1;
                        }
                        else {
                            debug_image.put_pixel(both_offset_x, both_offset_y, *image_1_pixel );
                        }
                    }
                }

                for block_y_pixel in 0..block_size {
                    for block_x_pixel in 0..block_size {
                        let both_offset_x = x_block_i * block_size + block_x_pixel;
                        let both_offset_y = y_block_i * block_size + block_y_pixel;
                        if both_offset_x >= overlap_info.overlap.0 || both_offset_y >= overlap_info.overlap.1 {
                            continue
                        }
                        let image_1_pixel = *img1.get_pixel(both_offset_x + overlap_info.img1_offset.0, both_offset_y + overlap_info.img1_offset.1);
                        let image_2_pixel = *img2.get_pixel(both_offset_x + overlap_info.img2_offset.0, both_offset_y + overlap_info.img2_offset.1);

                        /*debug_image.put_pixel(both_offset_x, both_offset_y,
                           if different_pixel_count != 0 {
                                compressor.write(&image_2_pixel.data).unwrap();
                                                              image::Rgba( [image_1_pixel[0]/2 + image_2_pixel[0]/2,
                                   image_1_pixel[1]/2 + image_2_pixel[1]/2,
                                   image_1_pixel[2]/2 + image_2_pixel[2]/2, 255] )

                                }
                           else {

                               image_1_pixel
                           }
                        );*/
                    }
                }

                //compress only changed blocks

                println!("Block ({},{}) has {} diff pix", x_block_i, y_block_i, different_pixel_count);
            }
        }
        let mut bitmap_bits: Vec<u8> = Vec::new();

        for byte_i in 0..bitmap.len()/8
        {
            let mut current_byte = 0;
            for bit_i in 0..8
            {
                current_byte |= (bitmap[byte_i * 8 + bit_i] << bit_i)
            }
            bitmap_bits.push(current_byte);
        }

        let mut bitmap_compressed: Vec<u8> = Vec::new();
        let brotli_quality = 11;
        let brotli_window = 24;
        {
            let mut compressor2 = brotli::CompressorWriter::new(
                &mut bitmap_compressed,
                BROTLI_BUFFER_SIZE,
                brotli_quality,
                brotli_window);

            //compressor2.write(&bitmap);
            compressor2.write(&bitmap_bits);
        }
        println!("Vector size is {}, compressed size is {}", bitmap.len(), bitmap_compressed.len());

        let save_path = std::path::Path::new("debug_images").join(&relative_paths[i]);
        std::fs::create_dir_all(save_path.parent().unwrap()).unwrap();
        debug_image.save(save_path);
    }
}

/// Saves a raw canvas image using an external compressor
/// compressor: the compressor to use to save the image
/// canvas_as_raw: the raw image to be saved using the compressor
fn save_brotli_image<T>(compressor : &mut brotli::CompressorWriter<T>, canvas_as_raw : &Vec<u8>, print_execution_time : bool) -> usize
where T: std::io::Write
{
    let brotli_start = time::PreciseTime::now();
    let bytes_written = compressor.write(canvas_as_raw).unwrap();
    let brotli_end = time::PreciseTime::now();
    if print_execution_time { println!("Brotli compression took {} seconds", brotli_start.to(brotli_end)); }
    return bytes_written;
}

//assume square block
//block offset - offset in blocks (NOT in pixels) to check for match
//returns number of non-matching pixels in block
//if 0 then do block copy
//if < threshold then do block subtract
//if > threshold then just do raw data
fn get_different_pixels_in_block(img1: &image::RgbaImage, img2: &image::RgbaImage, block_offset : (u32, u32), block_size : u32) //-> u64
{
    let start_x = block_offset.0 * block_size;
    let start_y = block_offset.1 * block_size;

    //note: end_x/y is the bound of the for loop, NOT the last index iterated by the for loop
    //let end_x = std::cmp::min(start_x + block_size, img)
}

pub fn alt_compression(brotli_archive_path : &str)
{
    let (max_width, max_height) = scan_folder_for_max_png_size("input_images");
    let canvas_width  = (max_width + 1)  & !0x1;
    let canvas_height = (max_height + 1) & !0x1;
    println!("max image size: ({},{})\nchosen canvas size: ({},{})", max_width, max_height, canvas_width, canvas_height);

    let mut base_images : Vec<RgbaImage> = Vec::new();
    let mut relative_paths : Vec<String> = Vec::new();

    let mut count = 0;
    for entry in WalkDir::new("input_images")
    {
        let mut canvas = RgbaImage::new(canvas_width, canvas_height);
        let ent = entry.unwrap();
        if ent.file_type().is_dir() {
            continue;
        }

        let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();

        relative_paths.push(String::from(path_relative_to_input_folder));

        let img_dyn = image::open(ent.path()).unwrap();
        let img = img_dyn.as_rgba8().unwrap();

        let offset_from_bottom_center = offset_to_bottom_center_image(&canvas, &img);
        canvas.copy_from(img, offset_from_bottom_center.0, offset_from_bottom_center.1);


        base_images.push(canvas);

        println!("Image Path: {}", path_relative_to_input_folder);
    }

    //add compressor here
    let brotli_quality = 11;
    let brotli_window = 24;
    let mut brotli_file = File::create("alt_image.brotli").expect("Cannot create file");
    let mut compressor = brotli::CompressorWriter::new(
    &brotli_file,
    BROTLI_BUFFER_SIZE,
    brotli_quality,
    brotli_window);

   /* for x in 0..canvas_width{
        println!("processing column {}", x);
        for y in 0..canvas_height{
            for i in 0..base_images.len()
            {
                let image = &base_images[i];
                let pixel_to_compress = image.get_pixel(x,y);
                //println!("w{:?}", pixel_to_compress);
                compressor.write(&pixel_to_compress.data).unwrap();
            }
        }
    }*/
    let target_block_size = 50;
    let x_num_normal_blocks = canvas_width / target_block_size + 1; //always do one extra, even if not needed (for now)
    let y_num_normal_blocks = canvas_height / target_block_size + 1; //always do one extra, even if not needed (for now)

    for i in 0..base_images.len() {
        let image = &base_images[i];
        for y_block_i in 0..y_num_normal_blocks
        {
            for x_block_i in 0..x_num_normal_blocks
            {
                for block_y_pixel_ind in 0..target_block_size {
                    for block_x_pixel_ind in 0..target_block_size {
                        let y_index = y_block_i * target_block_size + block_y_pixel_ind;
                        let x_index = x_block_i * target_block_size + block_x_pixel_ind;

                        //check in range
                        if x_index >= image.width() || y_index >= image.height() {
                            continue
                        }

                        let pixel_to_compress = image.get_pixel(x_index,y_index);
                        compressor.write(&pixel_to_compress.data).unwrap();
                    }
                }
            }
        }
    }

    /*let mut count = 0;
    for image in base_images
    {
        let save_path = std::path::Path::new("debug_images").join(&relative_paths[count]);
        std::fs::create_dir_all(save_path.parent().unwrap()).unwrap();

        image.save(save_path);

        compressor.write(&image.into_raw()).unwrap();
        count += 1;
    }*/



}

/// File format is as follows:
/// [8 bytes BigE 64u = X]  - a Big Endian, 64u value indicating the length of the CompressedFileInfo JSON at the end of the file
/// [Variable Length]       -  Brotli compressed image data.
/// [X bytes long]          - JSON encoded DecompressionInfo struct. Length given in first 4 bytes of the file.
//output_basename is the name of the brotli/metadatafiles, without the file extension (eg "a" will produce "a.brotli" and "a.metadata"
//Note: json is always stored uncompressed!
pub fn compress_path(brotli_archive_path : &str, use_json : bool, debug_mode : bool)
{
    let brotli_quality = 11;
    let brotli_window = 24;

    let (max_width, max_height) = scan_folder_for_max_png_size("input_images");
    let canvas_width  = (max_width + 1)  & !0x1;
    let canvas_height = (max_height + 1) & !0x1;
    println!("max image size: ({},{})\nchosen canvas size: ({},{})", max_width, max_height, canvas_width, canvas_height);

    let mut image_start_index_in_brotli_stream : usize = 0;
    let mut images_meta_info = Vec::new();
    let mut brotli_file = File::create(brotli_archive_path).expect("Cannot create file");

    //allocate some space to record how long the brotli compressed data is
    brotli_file.write(&[0; FILE_FORMAT_HEADER_LENGTH]).expect("Unable to allocate header space in file");

    //This set of braces forces the &f mutable reference used in the 'compressor' to go out of scope
    {
        let mut compressor = brotli::CompressorWriter::new(
        &brotli_file,
        BROTLI_BUFFER_SIZE,
        brotli_quality,
        brotli_window);

        let mut canvas = RgbaImage::new(canvas_width, canvas_height);

        println!("Begin scanning for images");

        //TODO: check that input_images directory exists before scanning it.
        //TODO: check each image's color type as subtracting a RGB image from an RGBA image shouldn't work.
        // see: println!("{:?}", img.color());

        let test_iter = WalkDir::new("input_images");
        let mut count = 0;
        for entry in test_iter
        {
            let ent = entry.unwrap();
            if ent.file_type().is_dir() {
                continue;
            }

            let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();

            println!("\nProcessing Image {}: '{}'", count, ent.path().display());
            let img_dyn = image::open(ent.path()).unwrap();
            let img = img_dyn.as_rgba8().unwrap();
            println!("Original Image width is {} height is {}", img.width(), img.height());

            //save raw image
            let img_as_vec = img.clone().into_raw();
            let save_path = std::path::Path::new("raw_images").join(path_relative_to_input_folder);
            std::fs::create_dir_all(save_path.parent().unwrap()).unwrap();
            std::fs::write(save_path, img_as_vec).expect("fail1");

            //try compressing original image
/*                let img_as_vec = img.clone().into_raw();
                let mut custom_image : Vec<u8> = Vec::new();//RgbaImage::new(img.width(), img.height());

                let channel_size = img_as_vec.len()/4;
                for index_in_channel in 0..channel_size
                {
                    custom_image.push(img_as_vec[index_in_channel*4 + 0]);
                    custom_image.push(img_as_vec[index_in_channel*4 + 1]);
                    custom_image.push(img_as_vec[index_in_channel*4 + 2]);
                }
                for index_in_channel in 0..channel_size
                {
                    custom_image.push(img_as_vec[index_in_channel*4 + 3]);
                }

                let original_image_compressed = compress_buffer(&custom_image);
               // let original_image_size = custom_image.width() * custom_image.height() * 4;
               // let image_size_percent = original_image_compressed.len() as f64 / original_image_size as f64;
               // println!("Compression of original image is [{}] bytes [{}%]", original_image_compressed.len(), image_size_percent);
                //save compressed image file to disk
               // custom_image.save("first_image.png");
                std::fs::write("first_image.brotli", original_image_compressed).expect("fail1");

                //save raw image file to disk
                //std::fs::write("first_image.raw", custom_image.clone().to_vec()).expect("fail2");
                return;*/


            if debug_mode { println!("Subtracting image from bottom center of canvas, then cropping image "); }
            let offset_from_bottom_center = offset_to_bottom_center_image(&canvas, &img);
            subtract_image_from_canvas(&mut canvas, &img, offset_from_bottom_center);
            //canvas.copy_from(img, offset_from_bottom_center.0, offset_from_bottom_center.1);

            let (cropped_image_bounds , num_identical_pixels) =
                crop_function(&canvas, offset_from_bottom_center, img.width(), img.height());
            let num_pixels_in_image = img.width() * img.height();
            let similarity = num_identical_pixels as f64 / num_pixels_in_image as f64 * 100.0;
            println!("Image is {}% similar to previous image", similarity);

            let cropped_image =
                image::imageops::crop(&mut canvas,
                cropped_image_bounds.x, cropped_image_bounds.y, //0,0,//
                cropped_image_bounds.width, cropped_image_bounds.height).to_image();
                //canvas_width, canvas_height).to_image();
            //try compressing cropped image
            /*{
                let diff_image_compressed = compress_image_to_buffer(&cropped_image);
                let diff_image_size = img.width() * img.height() * 4;
                let image_size_percent = diff_image_compressed.len() as f64 / diff_image_size as f64;
                println!("Compression of diff image is [{}] bytes [{}%]", diff_image_compressed.len(), image_size_percent);
                println!("Diff compression was {}% smaller than naive compression",  diff_image_compressed.len() as f64 / original_image_compressed.len() as f64);
            }*/


            //save meta info
            images_meta_info.push(CompressedImageInfo{
                start_index: image_start_index_in_brotli_stream,   //where in the compressed data stream the image starts
                x: cropped_image_bounds.x,                         //where on the canvas the diff should be placed (NEEDS UPDATE
                y: cropped_image_bounds.y,
                diff_width: cropped_image_bounds.width,            //the width and height of the diff image
                diff_height: cropped_image_bounds.height,
                output_width: img.width(),                         //the width and height of the reconstructed image
                output_height: img.height(),
                output_path: String::from(path_relative_to_input_folder),
            });

            image_start_index_in_brotli_stream += cropped_image.len();
            println!("Image size is {},  width is {} height is {}", cropped_image.len(), cropped_image_bounds.width, cropped_image_bounds.height);

            //save diff image as png for debugging reasons
            if debug_mode { save_image_no_alpha(cropped_image.clone(), &path_relative_to_input_folder); }

            // Compress the the diff image (or 'normal' image for first image)
            // NOTE: the below 'into_raw()' causes a move, so the canvas cannot be used anymore
            // However subsequent RgbaImage::new assigns a new value to the canvas each iteration
            let cropped_image_as_raw = cropped_image.into_raw();
            save_brotli_image(&mut compressor,
                               &convert_pixel_based_to_channel_based(cropped_image_as_raw, (cropped_image_bounds.width, cropped_image_bounds.height)),
                              true);

            // Prepare for next iteration by clearing canvas, then copying the 'original' image for the next diff
            canvas = RgbaImage::new(canvas_width, canvas_height);
            canvas.copy_from(img, offset_from_bottom_center.0, offset_from_bottom_center.1);


            count += 1;
        }
    }

    //Save decompression info to file, and record its length
    let decompression_info = DecompressionInfo {
        canvas_size: (canvas_width, canvas_height),
        images_info: images_meta_info,
    };

    //saving meta info
    let decompression_info_start = brotli_file.seek(SeekFrom::Current(0)).unwrap();
    println!("Decompression Info starts at position {}", decompression_info_start);

    let serialized : Vec<u8> =  if use_json {
        serde_json::to_vec(&decompression_info).unwrap()
    } else {
        bincode::serialize(&decompression_info).unwrap()
    };

    if use_json {
        brotli_file.write(&serialized).expect("Unable to write metadata file");
    }
    else {
        let mut compressor = brotli::CompressorWriter::new(
            &brotli_file,
            BROTLI_BUFFER_SIZE,
            brotli_quality,
            brotli_window);
        compressor.write(&serialized).expect("Unable to write decompression info (brotli compressed)");
    }

    //save some information about the compression before writing header at start of file
    let uncompressed_metadata_size =  serialized.len();
    let total_file_size = brotli_file.seek(SeekFrom::Current(0)).unwrap();
    let metadata_length_bytes = total_file_size - decompression_info_start;
    let metadata_as_percentage_of_total = metadata_length_bytes as f64 / total_file_size as f64 * 100.0;

    //return to start of file to write header info
    brotli_file.seek(SeekFrom::Start(0)).unwrap();
    brotli_file.write(&u64_to_u8_buf_little_endian(decompression_info_start)).expect("Unable to write header info to file");

    println!("\n\n ------------ Compression Finished! ------------");
    println!("Total archive size is {}mbytes", total_file_size as f64 / 1e6);
    println!("Metadata is {} kbytes ({} uncompressed), {}% of total",
             metadata_length_bytes as f64 / 1000.0,
             uncompressed_metadata_size as f64 / 1000.0,
             metadata_as_percentage_of_total);
}