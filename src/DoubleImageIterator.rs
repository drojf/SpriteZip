
//struct DoubleImageIterator<'s> {
//    original_image : &'s image::RgbaImage,
//    reference_image : &'s image::RgbaImage,
//    block_x : u32,
//    block_y : u32,
//    x: u32,
//    y: u32,
//    num_x_blocks : u32,
//    num_y_blocks : u32,
//    block_size : u32,
//}
//
//impl<'s> DoubleImageIterator<'s> {
//    fn new(original_image : &'s image::RgbaImage, reference_image : &'s image::RgbaImage) -> DoubleImageIterator<'s>
//    {
//        let block_size = 50;
//
//        let num_x_blocks = original_image.width()/block_size  + if original_image.width()  % block_size == 0 { 0 } else { 1 };
//        let num_y_blocks = original_image.height()/block_size + if original_image.height() % block_size == 0 { 0 } else { 1 };
//
//        DoubleImageIterator {
//            original_image,
//            reference_image,
//            block_x : 0,
//            block_y : 0,
//            x : 0,
//            y : 0,
//            num_x_blocks,
//            num_y_blocks,
//            block_size,
//        }
//    }
//}

//def pos(width, height, B, i):
//    pixels_per_block_row = B * width
//    y_block = i // pixels_per_block_row
//    print('y_block:',y_block)
//    pixels_in_previous_y_rows = y_block * pixels_per_block_row
//    print('pixels_in_previous_y_rows:',pixels_in_previous_y_rows)
//    block_height = min(B, height - B * y_block)
//    print('block_height:',block_height)
//    x_block = (i - pixels_in_previous_y_rows) // (B * block_height)
//    print('x_block:',x_block)
//    block_width = min(B, width - B * x_block)
//    print('block_width:',block_width)
//    i_block = i - pixels_in_previous_y_rows - x_block * (B * block_height)
//    print('i_block:',i_block)
//    return (i_block % block_width + x_block * B, i_block // block_width + y_block * B)

struct DoubleImageIterator<'s> {
    original_image : &'s image::RgbaImage,
    reference_image : &'s image::RgbaImage,
    block_x : u32,
    block_y : u32,
    x: u32,
    y: u32,
    num_x_blocks : u32,
    num_y_blocks : u32,
    block_size : u32,
}

impl<'s> DoubleImageIterator<'s> {
    fn new(original_image : &'s image::RgbaImage, reference_image : &'s image::RgbaImage) -> DoubleImageIterator<'s>
    {
        let block_size = 50;

        let num_x_blocks = original_image.width()/block_size  + if original_image.width()  % block_size == 0 { 0 } else { 1 };
        let num_y_blocks = original_image.height()/block_size + if original_image.height() % block_size == 0 { 0 } else { 1 };

        DoubleImageIterator {
            original_image,
            reference_image,
            block_x_pixel : 0,
            block_y_pixel : 0,
            x_block_i : 0,
            y_block_i : 0,
            num_x_blocks,
            num_y_blocks,
            block_size,
        }
    }
}

impl<'s> Iterator for DoubleImageIterator<'s>  {
    type Item = (u32, u32, image::Rgba<u8>, image::Rgba<u8>);

    //

    fn next(&mut self) -> Option<Self::Item>
    {
		//determine how to increment the pixel position variables
		let block_height = std::cmp::min(b, original_image.height() - b * y_block_i);
		let block_width = std::cmp::min(b, original_image.width() - b * x_block_i);

        self.block_x_pixel += 1;

        if self.block_x_pixel >= self.block_width {
            self.block_x_pixel = 0;
            self.block_y_pixel += 1;

            if self.block_y_pixel >= self.block_height {
                self.block_y_pixel = 0;
                self.x_block_i += 1;

                if self.x_block_i >= self.num_x_blocks {
                    self.x_block_i = 0;
                    self.y_block_i += 1;

                    if self.y_block_i >= self.num_y_blocks {
                        return None
                    }
                }
            }
        }
    }
	
	fn next(&mut self) -> Option<Self::Item>
    {
		let i = self.i;
		let B = self.block_size;
		let pixels_per_block = B * B;
		let pixels_per_block_row = B * B * blocks_per_row;
		
		let block_x = (i / pixels_per_block     ) % blocks_per_row;
		let block_y = (i / pixels_per_block_row );
		
		let x_offset_within_block = i % B;
		let y_offset_within_block = ((y / B) % B);

		let x_offset = x_offset_within_block + block_x * B
		let y_offset = y_offset_within_block + block_y * B;
    }
	
	//use this one?
	fn next(&mut self) -> Option<Self::Item>
    {
		let B = self.block_size;
		let pixels_per_block_row = B * original_image.width();
		
		let block_y = i / pixels_per_block_row;
		let pixels_in_previous_block_rows = block_y * pixels_per_block_row;
		let block_height =  std::cmp::min(B, original_image.height() - B * y_block);
		
		//for all rows except the last row, block_height == B. 
		//for last row, block_height = image.height() % B
		let pixels_in_current_block_row = i - pixels_in_previous_block_rows;
		let x_block = pixels_in_current_block_row / (B * block_height); 
		
		//for the very last block, both block height and block width will != B
		let i_block = pixels_in_current_block_row - x_block * (block_width * block_height);
		
		let x = i_block % block_width + x_block * B;
        let y = i_block / block_width + y_block * B;
	}	



    //Version using a single itartion variable
    /*fn next(&mut self) -> Option<Self::Item>
    {
        //constants per each image
        let B = 50;
        let pixels_per_block_row = B * original_image.width(); //pixels contained in a row of blocks (including 'small' block if it exists)

        //which y block (row) you are in. Then calculate the block height of the current row (only different for last row, which may be smaller)
        let y_block = i / pixels_per_block_row;
        let pixels_in_previous_y_rows = y_block * pixels_per_block_row;
        let block_height = std::cmp::min(B, original_image.height() - B * y_block);

        //subtract previous Y block rows so only need to consider current block row, then divide by block size. Then get block width (only different for last column)
        let x_block = (i - pixels_in_previous_y_rows) / (B * block_height);
        let block_width  = std::cmp::min(B, original_image.width() - B * x_block);

        //subtract away all previous pixels except for current block's pixels
        let i_block = i - pixels_in_previous_y_rows - x_block * (B * block_height);

        //convert current block's coordinates into absolute x/y values
        let x = i_block % block_width + x_block * B;
        let y = i_block / block_width + y_block * B;
    }*/
}
