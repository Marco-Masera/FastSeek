use std::io::{Seek, Write};
use std::fs::File;
use std::path::Path;
use std::io::{self};

pub enum FileWriter{
    Standard(io::BufWriter<File>)
}
impl FileWriter{
    pub fn get_writer(filename: String) -> FileWriter {
        let path = Path::new(&filename);
        let file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", filename, why),
            Ok(file) => file,
        };
        return FileWriter::Standard(io::BufWriter::new(file));
    }

    pub fn write_all(&mut self, buffer: &[u8]) -> Result<(), io::Error> {
        match self {
            FileWriter::Standard(writer) => writer.write_all(buffer),
        }
    }

    pub fn seek(&mut self, pos: io::SeekFrom) -> Result<u64, io::Error> {
        match self {
            FileWriter::Standard(writer) => writer.seek(pos),
        }
    }
}