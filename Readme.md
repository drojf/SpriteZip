# Spritezip

Compresses and Decompresses a series of sprite images (as .png files) into a difference/brotli compressed archive file. Designed to compress a series of images which are very similar to each other, where similar images are alphabetically close to one another, and where images are bottom-center aligned.

# Limitations/Notes

- The program will only compress .png files, and ignore all other files.
- The program only accepts RGBA .png files, not RGB .png files.
- It is assumed that the sprites which are similar are in alphabetical order. If the image order is scrambled, the compression will be very poor 
    - **TODO:** Sort images by their perceptual image hash, like the Python `imagehash` library, to group similar images together.
- This compression assumes that sprites are aligned bottom-center on the image (but does take into account different image sizes). If the sprites are not bottom-center aligned, you will get very poor compression. When compressing, check the difference indicator (`Diff: [percent]`) is as you expect for the images you are compressing.
- Compression speed depends on similarity of images - if images are not very similar, the compression will be very slow.
- oxipng can't take raw image data as input, so the raw image needs to be compressed to .png, fed into oxipng, decompressed, then compressed again on final extraction.
- Oxipng can use multiple cpu cores, but my portion of the program is single threaded, so it's slower than it should be.
- There is currently no versioning/backwards compatability system - you should keep the .exe you used for compression with the archive you are compressing. 

# Usage

Note: the program doesn't have proper command line settings (like specifying the input/output directories or output filename). This might be fixed in the future.

## Compression

To compress images, place the images in the 'input_images' directory adjacent to the executable (if it does not exist, running the program will create it for you). Use the command:

`spritezip compress`

If you place a folder structure with images, the entire folder structure and images will be preserved on extraction (but only .png files will be compressed).

This will create a `compressed_images.brotli` archive file.

## Extraction

When extracting png files, you can use the 'fast' method which results in larger output .png files, but is faster, or the 'slow' method, which results in smaller .png files, but is slower.

#### Fast Mode

Put the archive file (must be called `compressed_images.brotli`) adjacent to the executable, then run the program with no arguments:

`spritezip`

#### Slow Mode

Put the archive file (must be called `compressed_images.brotli`) adjacent to the executable, then run the program as follows:

`spritezip extract 2`

If you want to see how oxipng is optimizing the images, use:

`spritezip extract 2 debug`

Warning: This mode is very slow. Images will be about 20% smaller when compressing sprite data (depends on image content).

The number can be changed in the range 0-6, but 2 is recommended. See the oxipng documentation for what this number means (it is the -o paramete rof oxipng).

Images will be prodcued in the `output_images` directory.

## Verify

This will check that the raw image of the images in the `input_images` directory match the images in the `output_images` directory. If it detects images are the same except for the pixels which are 'invisible' (which are fully transparent/`alpha = 0`), it will give a warning message as opposed to treating it as an error. This is useful when using 'slow mode' for extraction - oxipng will remove color information from fully transparent images.

`spritezip verify`

## Alphablend

This takes images from the `input_images` directory, converts them to onscripter 'alphablend' format, and places them in the `output_images` directory.

`spritezip alphablend`

## Self-Test

This is equivalent to running the compression, extraction, and verify steps.

`spritezip selftest`

# Operation

#### Compression

To be filled in with more detail, but the high level algorithm is:

- Iterate over images `(null image, 0), (0,1), (1,2), (2,3)` etc... 
- Set the 'current' image as the second image in the tuple, previous image as the first element in the tuple
- Crop the images as much as possible such that it still contains the different pixels in it (for example, if the only difference between two sprites is the character is holding a sword, crop that part of the image). 
    - I don't think this step is particularly important, but it seems to improve the compression ratio slightly.
- Create a 'difference bitmap', the same dimensions of the 'current image' which is 1 where the pixels of the cropped image differ, and 0 where they are the same. 
    - 1 means 'save this pixel' and for the extractor 'use the saved pixel'
    - 0 means 'don't save this pixel' and for the extractor 'use the previous image pixel'
    - Compress this data using brotli compression. Compression seems to be better using a byte-array rather than a bit-array. 
    - If the images are different sizes, and the 'current image' pixel has no corresponding 'previous image' pixel, just mark the bitmap as 1 (save the pixel/use saved image data)  
- Save the pixels which are different using brotli compression as per the above bitmap
- Record the metadata (such crop region, output image dimensions, output path...) for each image, and compress using brotli compression

NOTE: the order in which the pixels are iterated over makes a difference in compression ratio! I have chosen to iterate over the images block-wise (currently 50pix blocks) so that large regions of color/empty areas are compressed together. I would like to use 'snake order' which would remove most discontinuities when traversing the image, but this isn't implemented yet.

The null image is required so that the first image is fully recorded. My implementation happens to work just fine with the null image being an image with zero width and zero height.

Initially I tried subtract each each pair of images to get their differece, but you can get poor compression ratios if the two images are completely different images. For those cases, you'd need to add a special case. The way I've chosen allows each image to be procesed in the same manner, rather than having special cases for when images are too different.

#### Extraction

Probably can be guessed from the compression algorithm above. The main different part is that extraction can optimize the output .png files using oxipng. To be filled in later. 

Also of note is that you must use the same pixel traversal order as the compression stage, otherwise, the image will be scrambled :S.

