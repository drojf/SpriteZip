/// From https://stackoverflow.com/questions/42187591/how-to-keep-track-of-how-many-bytes-written-when-using-stdiowrite
//let mut compressed_bitmap_wrapper = ByteCounter::new(&mut compressed_bitmap_data_vector , 1_000_000);
//let mut bitmap_compressor = brotli::CompressorWriter::new(compressed_bitmap_wrapper, BROTLI_BUFFER_SIZE, brotli_quality, brotli_window);

use std::io::{self, Write, Seek, SeekFrom};

pub struct ByteCounter<W> {
    inner: W,
    count: usize,
    bytes_seen: usize,
    print_size: usize,
}

impl<W> ByteCounter<W>
    where W: Write
{
    pub fn new(inner: W, print_size : usize) -> Self {
        ByteCounter {
            inner: inner,
            count: 0,
            bytes_seen: 0,
            print_size,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn bytes_written(&self) -> usize {
        self.count
    }
}

impl<W> Write for ByteCounter<W>
    where W: Write
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let res = self.inner.write(buf);
        if let Ok(size) = res {
            self.bytes_seen += size;
            if self.bytes_seen > self.print_size {
                self.bytes_seen = 0;
                println!("Mem: {}", pretty_print_bytes(self.count as f64))
            }

            self.count += size
        }
        res
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
