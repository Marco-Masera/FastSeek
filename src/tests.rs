mod file_reader;
mod file_writer;
mod header;

use std::fs::File;
use std::io::{self, BufRead, Seek};
use std::path::Path;
use std::io::Write;
use std::vec;
use bgzip::{BGZFReader, BGZFWriter, Compression};
use stable_hash::fast_stable_hash;
use file_reader::FileReader;
use file_writer::FileWriter;

pub fn run_test(){
    let path = Path::new("test.csv");
    let file = File::create(&path).unwrap();
    let mut writer = io::BufWriter::new(file);
    for i in 0..100 {
        let string = format!("prova{}", i);
        let _ = writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
        //writer.write_line();
    }
    let _ = writer.flush();
    index("test.csv".to_string(), 1, 0, ",".to_string());
    for i in 0..100 {
        search(format!("prova{}", i), "test.csv".to_string(), 1, ",".to_string());
    }
}
pub fn run_test_compressed(){
    let path = Path::new("test.csv.gz");
    let file = File::create(&path).unwrap();
    let buf_writer = io::BufWriter::new(file);
    let mut writer = BGZFWriter::new(buf_writer, Compression::default());
    for i in 0..100 {
        let string = format!("prova{}", i);
        let _ =writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
        //writer.write_line();
    }
    let _ = writer.flush();
    let _ = writer.close();
    index("test.csv.gz".to_string(), 1, 0, ",".to_string());
    for i in 0..100 {
        search(format!("prova{}", i), "test.csv.gz".to_string(), 1, ",".to_string());
    }
}
pub fn test_compression(){
    //Write some lines
    let path = Path::new("test.gz");
    let file = File::create(&path).unwrap();
    let buf_writer = io::BufWriter::new(file);
    //let mut write_buffer = Vec::new();
    let mut writer = BGZFWriter::new(buf_writer, Compression::default());
    for i in 0..10 {
        let string = format!("prova{}", i);
        let _ = writer.write_all(format!("1,{},0,0,0\n", string).as_bytes());
    }
    let _ = writer.close();

    //Read all the lines
    let file = File::open(&path).unwrap();
    let buf_reader = io::BufReader::new(file);

    let mut reader = BGZFReader::new(buf_reader).unwrap();
    let mut offset = 0;
    let mut v = Vec::new();
    loop{
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).unwrap();
        if bytes_read == 0 {
            break;
        }
        println!("First read - {}: {}",offset, line);
        v.push(offset);
        offset += bytes_read;
    }
    for i in (0..10).rev() {
        let _ = reader.bgzf_seek(v[i] as u64);
        let mut line = String::new();
        let _ = reader.read_line(&mut line).unwrap();
        println!("Second read: {}: {}",v[i], line);
    }
}
