use std::io::Read;
use std::fs::File;
use std::path::Path;
use bgzip::BGZFReader;
use std::io::{self, BufRead, Seek};

pub enum FileReader {
    Standard(io::BufReader<File>),
    BGZF(BGZFReader<io::BufReader<File>>),
}
impl FileReader {
    pub fn get_reader(filename: &String) -> FileReader {
        let path = Path::new(&filename);
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", filename, why),
            Ok(file) => file,
        };
        FileReader::Standard(io::BufReader::new(file))
    }
    pub fn get_gz_reader(filename: &String) -> FileReader {
        let file = File::open(&filename).unwrap();
        let buf_reader = io::BufReader::new(file);
        FileReader::BGZF(BGZFReader::new(buf_reader).unwrap())
    }

    pub fn seek(&mut self, pos: u64) -> () {
        match self {
            FileReader::Standard(reader) => _ = reader.seek(io::SeekFrom::Start(pos)),
            FileReader::BGZF(reader) => _ = reader.bgzf_seek(pos).unwrap(),
        };
    }
    pub fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), io::Error> {
        match self {
            FileReader::Standard(reader) => reader.read_exact(buffer),
            FileReader::BGZF(reader) => reader.read_exact(buffer),
        }
    }
    pub fn read_line(&mut self, buffer: &mut String) -> Result<usize, io::Error> {
        match self {
            FileReader::Standard(reader) => reader.read_line(buffer),
            FileReader::BGZF(reader) => reader.read_line(buffer),
        }
    }
    pub fn num_lines(&mut self) -> u64 {
        let mut number_lines = 0;
        match self {
            FileReader::Standard(reader) => {
                let mut buffer = [0; 8192];
                while let Ok(n) = reader.read(&mut buffer) {
                    if n == 0 { break; }
                    number_lines += buffer[..n].iter()
                        .filter(|&&byte| byte == b'\n')
                        .count();
                }
                self.seek(0);
            },
            FileReader::BGZF(reader) => {
                let mut buffer = [0; 8192];
                while let Ok(n) = reader.read(&mut buffer) {
                    if n == 0 { break; }
                    number_lines += buffer[..n].iter()
                        .filter(|&&byte| byte == b'\n')
                        .count();
                }
                self.seek(0);
            },
        }
        return number_lines as u64;
    }
}
