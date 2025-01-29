mod file_reader;
mod file_writer;

use std::fs::File;
use std::io::{self, BufRead, Seek};
use std::path::Path;
use std::io::Write;
use std::vec;
use bgzip::{BGZFReader, BGZFWriter, Compression};
use stable_hash::fast_stable_hash;
use file_reader::FileReader;
use file_writer::FileWriter;


fn hash_function(value: &str, hashmap_size: u128) -> u64 {
    let r = fast_stable_hash(&value) % hashmap_size;
    return r as u64;
}

fn add_to_output_blocks(output_writer: &mut FileWriter, position: u64, value: u64, next: u64) {
    output_writer.seek(io::SeekFrom::Start(position)).unwrap();
    output_writer.write_all(&value.to_ne_bytes()).unwrap();
    output_writer.write_all(&next.to_ne_bytes()).unwrap();
}


fn index(filename: String, column: usize, mut hashmap_size: u128, separator: String) {
    let is_compressed = filename.ends_with(".gz");
    let mut input_reader: FileReader = match is_compressed{
        true => FileReader::get_gz_reader(&filename),
        false => FileReader::get_reader(&filename),
    };
    if hashmap_size == 0 {
        hashmap_size = input_reader.num_lines() as u128;
    }
    let mut output_writer = FileWriter::get_writer(format!("{}.index", filename));

    //Write hashmap_size of 0s to index file
    output_writer.write_all(&hashmap_size.to_ne_bytes()).unwrap();
    const CHUNK_SIZE: usize = 1024*8;
    let total_bytes = hashmap_size as usize * 8;
    let mut remaining = total_bytes;
    let chunk = vec![0; CHUNK_SIZE];
    while remaining > 0 {
        let to_write = remaining.min(CHUNK_SIZE);
        output_writer.write_all(&chunk[..to_write]).unwrap();
        remaining -= to_write;
    }
    let mut block_start: u64 = (hashmap_size  as u64 +1)*8;


    let mut index_map: Vec<u64> = (0..hashmap_size).map(|_| 0).collect::<Vec<_>>();

    let mut line = String::new();
    let mut offset: u64 = 0;
    loop {
        let bytes_read = input_reader.read_line(&mut line).unwrap();
        if bytes_read == 0 {
            break;
        }

        let mut parts = line.split(&separator);
        let value = parts.nth(column).unwrap();
        let hash = hash_function(value, hashmap_size);
        add_to_output_blocks(&mut output_writer, block_start, offset, index_map[hash as usize]);
        index_map[hash as usize] = block_start;
        block_start += 16;

        offset += bytes_read as u64;
        line.clear();
    }
    output_writer.seek(io::SeekFrom::Start(8)).unwrap();
    for i in 0..hashmap_size {
        output_writer.write_all(&index_map[i as usize].to_ne_bytes()).unwrap();
    }
}

fn search(keyword: String, filename: String, column: usize, separator: String){
    let is_compressed = filename.ends_with(".gz");
    let mut file_reader: FileReader = match is_compressed{
        true => FileReader::get_gz_reader(&filename),
        false => FileReader::get_reader(&filename),
    };
    let mut index_reader = FileReader::get_reader(&format!("{}.index", filename));
    //Find hashmap size
    let mut buffer = [0; 8];
    index_reader.read_exact(&mut buffer).unwrap();
    let hashmap_size = u64::from_ne_bytes(buffer) as u128;
    
    //Hash keyword
    let hash = hash_function(&keyword, hashmap_size) + 1;
    //Find block start
    let mut block_start;
    index_reader.seek(hash*8);
    let mut buffer = [0; 8];
    index_reader.read_exact(&mut buffer).unwrap();
    block_start = u64::from_ne_bytes(buffer);
    loop{
        if block_start == 0 {
            println!("Keyword not found");
            break;
        }
        //Read offset and next
        let mut buffer = [0; 16];
        index_reader.seek(block_start);
        index_reader.read_exact(&mut buffer).unwrap();
        let offset = u64::from_ne_bytes(buffer[0..8].try_into().unwrap());
        let next = u64::from_ne_bytes(buffer[8..16].try_into().unwrap());
        //Read line in original file
        file_reader.seek(offset);
        let mut line = String::new();
        file_reader.read_line(&mut line).unwrap();
        let mut parts = line.split(&separator);
        let value = parts.nth(column).unwrap();
        if value == keyword {
            println!("{}", line);
            break;
        }
        block_start = next;
    }
}

fn run_test(){
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
fn run_test_compressed(){
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
fn test_compression(){
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


fn main() {
    run_test_compressed();
    //test_compression();
    //run_test();
    //index("data.csv".to_string(), 0, 0, ",".to_string());
    //search("prova2".to_string(), "data.csv".to_string(), 0, 0, ",".to_string());
}
