//standard uses
use std;
use std::io::{Write};
use std::fs::File;
use std::io::SeekFrom;
use std::io::Seek;
use std::iter::Iterator;

//non-standard use
use bincode;

use brotli;
use image;
use walkdir;
use walkdir::WalkDir;

use common::{pretty_print_bytes, pretty_print_percent};
use common::{CompressedImageInfo, DecompressionInfo};
use common::u64_to_u8_buf_little_endian;
use common::{FILE_FORMAT_HEADER_LENGTH, BROTLI_BUFFER_SIZE};
use common::get_offset_to_other_image;
use common::BlockXYIterator;
use common::try_get_pixel;

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

struct FileTypeIterator<'s> {
    walkdir_iterator : walkdir::IntoIter,
    file_type : &'s str,
}

impl<'s> FileTypeIterator<'s> {
    fn new(root : &'s str, file_type : &'s str) -> FileTypeIterator<'s>
    {
        FileTypeIterator {
            walkdir_iterator : WalkDir::new(root).into_iter(),
            file_type : file_type,
        }
    }
}

impl<'s> Iterator for FileTypeIterator<'s>  {
type Item = (walkdir::DirEntry);
	fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            let entry = match self.walkdir_iterator.next() {
                None => return None,
                Some(ent) => ent.unwrap(),
            };

            let is_file = entry.file_type().is_file();

            let extension_matches = match entry.path().extension() {
                None => false,
                Some(ext) => ext == self.file_type,
            };

            if is_file && extension_matches {
                return Some(entry);
            }
            else
            {
                continue
            }
        }
	}
}

pub struct BlockImageIterator<'s> {
    original_image : &'s image::RgbaImage,
    xy_iter : BlockXYIterator,
}

impl<'s> BlockImageIterator<'s> {
    fn new(original_image : &'s image::RgbaImage, block_size : usize) -> BlockImageIterator<'s>
    {
        BlockImageIterator {
            original_image,
            xy_iter : BlockXYIterator::new(block_size, (original_image.width() as usize, original_image.height() as usize)),
        }
    }
}

impl<'s> Iterator for BlockImageIterator<'s>  {
type Item = (u32, u32, image::Rgba < u8 >);
	//use this one?
	fn next(&mut self) -> Option<Self::Item>
    {
        match self.xy_iter.next() {
            None => None,
            Some((x,y)) => Some((x,y, *self.original_image.get_pixel(x, y))),
        }
	}
}


pub fn alt_compression_2(brotli_archive_path : &str)
{
    let brotli_quality = 11;
    let brotli_window = 24;

    //Create object to store all image metadata (but not the global metadata)
    let mut images_info : Vec<CompressedImageInfo> = Vec::new();

    let mut compressed_bitmap_data_vector = Vec::new();

    let mut archive_file = File::create(brotli_archive_path).expect("Cannot create file");

    //Allocate some space for the file format header
    archive_file.write(&[0; FILE_FORMAT_HEADER_LENGTH]).expect("Unable to allocate header space in file");

    //scope for compression/file objects (most of the work)
    {
        //Create compressors for image data and bitmap
        let mut image_compressor = brotli::CompressorWriter::new(&archive_file, BROTLI_BUFFER_SIZE, brotli_quality, brotli_window);
        let mut bitmap_compressor = brotli::CompressorWriter::new(&mut compressed_bitmap_data_vector, BROTLI_BUFFER_SIZE, brotli_quality, brotli_window);

        let mut prev_image = image::RgbaImage::new(0,0);

        for ent in FileTypeIterator::new("input_images", "png") {
            let path_relative_to_input_folder = ent.path().strip_prefix("input_images").unwrap().to_str().unwrap();

            let img_dyn = image::open(ent.path()).unwrap();
            let image = img_dyn.as_rgba8().unwrap();

            println!("Image Path: {}", path_relative_to_input_folder);

            let crop_region = alt_compression_3_inner(&image, &prev_image, &mut image_compressor, &mut bitmap_compressor);
            images_info.push(CompressedImageInfo {
                start_index: 0, //not used
                x: crop_region.top_left.0,
                y: crop_region.top_left.1,
                diff_width: crop_region.dimensions.0,
                diff_height: crop_region.dimensions.1,
                output_width: image.width(),
                output_height: image.height(),
                output_path: path_relative_to_input_folder.to_string(),
            });

            prev_image = image.clone(); //TODO: remove this clone?
        }
    }

    //Save the already compressed bitmap, recording where it starts in the file
    let bitmap_data_start = archive_file.seek(SeekFrom::Current(0)).unwrap();
    archive_file.write_all(&compressed_bitmap_data_vector).unwrap();

    //Compress and save the metadata, recording the start location in the file
    let metadata_start = archive_file.seek(SeekFrom::Current(0)).unwrap();
    let decompression_info = DecompressionInfo {
        canvas_size: (0, 0), //TODO: remove this - it's not used
        bitmap_data_start,
        images_info,
    };
    let serialized_metadata = bincode::serialize(&decompression_info).unwrap();
    {
        brotli::CompressorWriter::new(&archive_file, BROTLI_BUFFER_SIZE, brotli_quality, brotli_window)
            .write_all(&serialized_metadata).unwrap();
    }

    //save end of file location
    let file_size = archive_file.seek(SeekFrom::Current(0)).unwrap();

    //return to start of file to write metadata offset
    archive_file.seek(SeekFrom::Start(0)).unwrap();
    archive_file.write(&u64_to_u8_buf_little_endian(metadata_start)).expect("Unable to write header offset to file");

    //Print debug information
    let bitmap_data_length = metadata_start - bitmap_data_start;
    let metadata_length_bytes = file_size - metadata_start;

    println!("\n\n ------------ Compression Finished! ------------");
    println!("Total archive size is {}", pretty_print_bytes(file_size as f64));
    println!("Bitmap data is {}, {} of total",
             pretty_print_bytes(bitmap_data_length as f64),
             pretty_print_percent(bitmap_data_length, file_size));

    println!("Metadata is {} ({} uncompressed), {} of total",
             pretty_print_bytes(metadata_length_bytes as f64),
             pretty_print_bytes(serialized_metadata.len() as f64),
             pretty_print_percent(metadata_length_bytes, file_size));
}

pub fn alt_compression_3_inner<'s,T,V>(original_image : &image::RgbaImage, prev_image : &image::RgbaImage, image_compressor : &'s mut brotli::CompressorWriter<T>, bitmap_compressor : &'s mut   brotli::CompressorWriter<V>) -> CropRegion
where T: std::io::Write,
      V: std::io::Write
{
    let (x_offset_to_other_image , y_offset_to_other_image)= get_offset_to_other_image(original_image, prev_image);

    // ----------------------------  DO CROP  ----------------------------
    let mut cropper = Cropper::new((original_image.width(), original_image.height()));
    let mut debug_difference_count = 0;
    for (x, y, original_image_pixel) in original_image.enumerate_pixels()
    {
        let prev_x = x as i64 + x_offset_to_other_image;
        let prev_y = y as i64 + y_offset_to_other_image;

        let pixels_equal = match try_get_pixel((prev_x, prev_y), &prev_image) {
            None => false,
            Some(prev_pixel) => *original_image_pixel == prev_pixel,
        };

        if !pixels_equal {
            cropper.add_nonzero_pixel(x, y);
            debug_difference_count += 1;
        }
    }

    //Get a cropped version of the image to work on
    let crop_region = cropper.get_crop_region();
    println!("Images are {} identical. {:?}", pretty_print_percent(debug_difference_count, original_image.width() as u64 * original_image.height() as u64), crop_region);

    let cropped_image = image::imageops::crop(&mut original_image.clone(),
    crop_region.top_left.0, crop_region.top_left.1,
    crop_region.dimensions.0, crop_region.dimensions.1).to_image();

    println!("Actual image dimensions (should match) {} {} num pixels {}", cropped_image.width(), cropped_image.height(), cropped_image.width() * cropped_image.height());

    // ----------------------------  DO COMPRESS  ----------------------------
    let mut debug_difference_count = 0;
    let mut difference : Vec<u8> = Vec::with_capacity(cropped_image.width() as usize * cropped_image.height() as usize);

    for (x,y,cropped_pixel) in BlockImageIterator::new(&cropped_image, 50)
    {
        let original_image_x = x + crop_region.top_left.0;
        let original_image_y = y + crop_region.top_left.1;
        let prev_x = original_image_x as i64 + x_offset_to_other_image;
        let prev_y = original_image_y as i64 + y_offset_to_other_image;

        let pixels_equal = match try_get_pixel((prev_x, prev_y), &prev_image) {
            None => false,
            Some(prev_pixel) => cropped_pixel == prev_pixel,
        };

        if pixels_equal {
            difference.push(0u8);
            debug_difference_count += 1;
        }
        else {
            difference.push(1u8);
            image_compressor.write(&cropped_pixel.data).unwrap();
        }
    }

    println!("bitmap difference size is {}", difference.len());

    bitmap_compressor.write(&difference).unwrap();

    //return crop_region to be saved as metadata
    return crop_region
}

//new image format:
// format 1
// format                              data name                       description
//---------------------------------------------------------------------------------------------------
//[u64]                                 metadata_start_index
//[brotli compresed Vec<u8>]            compressed_image_bitmap_1       - uncompressed size is crop_region_width * crop_region_height * 1 (one byte per pixel)
//[brotli compressed image Rgba<u8>]    compressed_image_data_1         - uncompressed size is equal to the number of '1's in the bitmap
//                                      compressed_image_bitmap_2
//                                      compressed_image_data_2
// ...more images go here...
//[brotli compressed metadata struct]   metadata struct for everything  - (holds start locations of each compressed image data and bitmap
//                                                                        Must read this first to decode images

//format2
// format                              data name                       description
//---------------------------------------------------------------------------------------------------
//(optional? general image data at start of file?
//[brotli compressed image metadata]    compressed_image_metadata_1     - use bincode::deserialize_from to get struct out of compressed data.
//[brotli compresed Vec<u8>]            compressed_image_bitmap_1       - uncompressed size is crop_region_width * crop_region_height * 1 (one byte per pixel)
//[brotli compressed image Rgba<u8>]    compressed_image_data_1         - uncompressed size is equal to the number of '1's in the bitmap
//                                      compressed_image_metadata_2
//                                      compressed_image_bitmap_2
//                                      compressed_image_data_2
// ...more images go here...
// to ensure compression effiency:
// For this method, should make three compressors backed by vecs.
// For each iteration,
//      run the algorithm to save compressed data to the three vectors
//      dump the vectors into the file

//format3
// format                              data name                       description
//---------------------------------------------------------------------------------------------------
//[u64]                                 metadata_start_index
//[brotli compresed Vec<u8>]            compressed_image_bitmap_ALL       - uncompressed size is crop_region_width * crop_region_height * 1 (one byte per pixel)
//[brotli compressed image Rgba<u8>]    compressed_image_data_ALL         - uncompressed size is equal to the number of '1's in the bitmap
//[brotli compressed imageS metadata]   compressed_image_metadata_1       - use bincode::deserialize_from to get struct out of compressed data.

// ...more images go here...
// to ensure compression effiency:
// For this method, should make the big image data backed by a file, and the remaining two backed by Vec<u8> (they should be small, even with 10,000 images. Should print this out to check size
// For each iteration,
//    compress the
//


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
