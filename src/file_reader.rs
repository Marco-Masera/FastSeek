use std::io::Read;
use std::fs::File;
use std::path::Path;
use bgzip::BGZFReader;
use std::io::{self, BufRead, Seek};


pub trait FileReader{
    fn seek(&mut self, pos: u64) -> ();
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), io::Error>;
    fn read_line(&mut self, buffer: &mut String) -> Result<usize, io::Error>;
    fn num_lines(&mut self) -> u64;
}

pub struct StandardFileReader{
    buf_reader: io::BufReader<File>
}
impl StandardFileReader{
    pub fn new(filename: &String) -> StandardFileReader{
        let path = Path::new(&filename);
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", filename, why),
            Ok(file) => file,
        };
        return StandardFileReader{buf_reader: io::BufReader::new(file)};
    }
}

impl FileReader for StandardFileReader{
    fn seek(&mut self, pos: u64) -> () {
        let _ = self.buf_reader.seek(io::SeekFrom::Start(pos));
    } 
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), io::Error> {
        self.buf_reader.read_exact(buffer)
    }
    fn read_line(&mut self, buffer: &mut String) -> Result<usize, io::Error> {
        self.buf_reader.read_line(buffer)
    }
    fn num_lines(&mut self) -> u64 {
        let mut number_lines = 0;
        let mut buffer = [0; 8192];
        while let Ok(n) = self.buf_reader.read(&mut buffer) {
            if n == 0 { break; }
            number_lines += buffer[..n].iter()
                .filter(|&&byte| byte == b'\n')
                .count();
        }
        self.seek(0);
        return number_lines as u64;
    }
}

pub struct GzFileReader{
    bgzf_reader: BGZFReader<io::BufReader<File>>
}
impl GzFileReader{
    pub fn new(filename: &String) -> GzFileReader {
        let file = File::open(&filename).unwrap();
        let buf_reader = io::BufReader::new(file);
        return GzFileReader{bgzf_reader: BGZFReader::new(buf_reader).unwrap()};
    }
}

impl FileReader for GzFileReader{
    fn seek(&mut self, pos: u64) -> () {
        self.bgzf_reader.bgzf_seek(pos).unwrap();
    } 
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), io::Error> {
        self.bgzf_reader.read_exact(buffer)
    }
    fn read_line(&mut self, buffer: &mut String) -> Result<usize, io::Error> {
        self.bgzf_reader.read_line(buffer)
    }
    fn num_lines(&mut self) -> u64 {
        let mut number_lines = 0;
        let mut buffer = [0; 8192];
        while let Ok(n) = self.bgzf_reader.read(&mut buffer) {
            if n == 0 { break; }
            number_lines += buffer[..n].iter()
                .filter(|&&byte| byte == b'\n')
                .count();
        }
        self.seek(0);
        return number_lines as u64;
    }
}


pub trait InputReader{
    //Returns offset of the entry and the indexing keyword
    fn get_entry(&mut self, buffer: &mut String) -> usize;
    //Test entry with value, returns true if found and set the entire entry to the buffer if found
    //if test fails, buffer is left dirty - caller must clear it
    fn test_and_return_entry(&mut self, offset: u64, value: &String, buffer: &mut String) -> bool;
    fn reset(&mut self);
    fn num_entries(&mut self) -> u64;
}

pub struct TabularInputReader<'a>{
    file_reader: &'a mut dyn FileReader,
    offset: usize,
    separator: &'a str,
    column: usize
}
impl<'a> TabularInputReader<'a>{
    pub fn new(file_reader: &'a mut dyn FileReader, separator: &'a str, column: usize) -> TabularInputReader<'a>{
        return TabularInputReader{file_reader:file_reader, offset:0, separator:separator, column:column};
    }
}

impl InputReader for TabularInputReader<'_>{
    fn get_entry(&mut self, buffer: &mut String) -> usize{
        //read entire line - take advantage of user-provided buffer to store it
        let return_value = self.offset;
        let bytes_read = self.file_reader.read_line(buffer).unwrap();
        if bytes_read == 0{
            return 0xFFFFFFFFFFFFFFFF;
        };
        self.offset = self.offset + bytes_read;
        //Split and return only the column of interest
        let mut parts = (& buffer).split(&self.separator);
        let value = parts.nth(self.column).unwrap().to_string();
        buffer.clear();
        buffer.push_str(&value);
        return return_value;
    }
    fn reset(&mut self){
        self.file_reader.seek(0);
        self.offset = 0;
    }
    fn num_entries(&mut self) -> u64{
        return self.file_reader.num_lines();
    }
    fn test_and_return_entry(&mut self, offset: u64, value: &String, buffer: &mut String) -> bool{
        self.file_reader.seek(offset);
        self.file_reader.read_line(buffer).unwrap();
        let mut parts = buffer.split(self.separator);
        let key = parts.nth(self.column).unwrap();
        return key == value;
    }
}