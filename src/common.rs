pub struct Rectangle  {
    pub width: u32,
    pub height: u32,
    pub brotli_buffer_size: usize,
    pub brotli_quality: u32,
    pub brotli_window: u32,
}

pub static CANVAS_SETTING : Rectangle = Rectangle {
    width : 3000,
    height: 3000,
    brotli_buffer_size: 4096,
    brotli_quality: 9, //11, //9 seems to be a good tradeoff...changing q doesn't seem to make much diff though?
    brotli_window: 22
};

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct CompressedImageInfo {
    pub start_index: usize,
    //where in the compressed data stream the image starts
    pub x: u32,
    //where on the canvas the diff should be placed
    pub y: u32,
    pub diff_width: u32,
    //the width and height of the diff image
    pub diff_height: u32,
    pub output_width: u32,
    //the width and height of the reconstructed image
    pub output_height: u32,
}
