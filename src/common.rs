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
