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


const CURRENT_VERSION: u8 = 0;
const HASHMAP_ENTRY_SIZE: u8 = 8;


fn hash_function(value: &str, hashmap_size: u128) -> u64 {
    let r = fast_stable_hash(&value) % hashmap_size;
    return r as u64;
}

#[derive(PartialEq, Eq)]
enum IndexEntryType{
    Direct,
    Indirect,
    NULL
}
struct IndexEntry{
    index: u64
}
impl IndexEntry{
    fn get_type(&self) -> IndexEntryType{
        if self.index == u64::MAX {
            return IndexEntryType::NULL;
        }
        if self.index & 0x8000000000000000 == 0{
            return IndexEntryType::Direct;
        }
        return IndexEntryType::Indirect;
    }
    fn get_offset(&self) -> u64{
        return self.index & 0x7FFFFFFFFFFFFFFF;
    }
    fn new_direct(offset: u64) -> IndexEntry{
        return IndexEntry{index: offset};
    }
    fn new_indirect(offset: u64) -> IndexEntry{
        return IndexEntry{index: offset | 0x8000000000000000};
    }
    fn new_null() -> IndexEntry{
        return IndexEntry{index: u64::MAX};
    }
    fn to_be_bytes(&self) -> [u8; 8]{
        return self.index.to_be_bytes();
    }
    fn from_be_bytes(bytes: [u8; 8]) -> IndexEntry{
        return IndexEntry{index: u64::from_be_bytes(bytes)};
    }
}

fn add_to_output_blocks(output_writer: &mut FileWriter, position: u64, value: u64, next: u64) {
    output_writer.seek(io::SeekFrom::Start(position)).unwrap();
    output_writer.write_all(&value.to_be_bytes()).unwrap();
    output_writer.write_all(&next.to_be_bytes()).unwrap();
}


fn index(filename: String, column: usize, mut hashmap_size: u128, separator: String) {
    let is_compressed = filename.ends_with(".gz");
    //Create reader
    let mut input_reader: FileReader = match is_compressed{
        true => FileReader::get_gz_reader(&filename),
        false => FileReader::get_reader(&filename),
    };

    //If unspecified, set hashmap_size to number of lines
    if hashmap_size == 0 {
        hashmap_size = input_reader.num_lines() as u128;
    }

    //Create header object
    let header = header::Header::new(CURRENT_VERSION, hashmap_size as u64);

    //Create writer
    let mut output_writer = FileWriter::get_writer(format!("{}.index", filename));

    //Write header to index file
    output_writer.write_all(&header.to_bytes()).unwrap();
    //Write empty hashmap
    const CHUNK_SIZE: usize = 1024*8;
    let total_bytes = hashmap_size as usize * (HASHMAP_ENTRY_SIZE as usize);
    let mut remaining = total_bytes;
    let chunk = vec![255; CHUNK_SIZE];
    while remaining > 0 {
        let to_write = remaining.min(CHUNK_SIZE);
        output_writer.write_all(&chunk[..to_write]).unwrap();
        remaining -= to_write;
    }

    //Keep indexes where to write the blocks
    let block_starting_address: u64 = (hashmap_size as u64 * HASHMAP_ENTRY_SIZE as u64) + (header.get_header_size() as u64);
    let mut block_first_free: u64 = block_starting_address;
    //In-memory structure for the hashmap (TODO: might have to do on-disk)
    let mut index_map: Vec<IndexEntry> = (0..hashmap_size).map(|_| IndexEntry::new_null()).collect::<Vec<_>>();

    let mut line = String::new();
    let mut offset_on_original_file: u64 = 0;
    loop {
        let bytes_read = input_reader.read_line(&mut line).unwrap();
        if bytes_read == 0 {
            break;
        }
        let mut parts = line.split(&separator);
        let value = parts.nth(column).unwrap();
        let hash = hash_function(value, hashmap_size);
        
        /*
            offset_on_original_file: offset in the original file where the line was read
            hash: index on has table
            block_first_free: offset in the block part of index
            index_map: hashmap that maps hash to block offset
         */
        let current_index = &index_map[hash as usize];
        match current_index.get_type(){
            IndexEntryType::NULL => {index_map[hash as usize] = IndexEntry::new_direct(offset_on_original_file);}
            IndexEntryType::Indirect => {
                let next = current_index.get_offset();
                add_to_output_blocks(
                    &mut output_writer, block_first_free, 
                    offset_on_original_file, current_index.index
                );
                index_map[hash as usize] = IndexEntry::new_indirect(block_first_free);
                block_first_free += 16;
            }
            IndexEntryType::Direct => {
                let value = current_index.get_offset();
                add_to_output_blocks(
                    &mut output_writer, block_first_free, 
                    value, IndexEntry::new_indirect(block_first_free+16).index
                );
                index_map[hash as usize] = IndexEntry::new_indirect(block_first_free);
                block_first_free += 16;
                add_to_output_blocks(
                    &mut output_writer, block_first_free, 
                    offset_on_original_file, IndexEntry::new_null().index 
                );
                block_first_free += 16;
            }
        }

        offset_on_original_file += bytes_read as u64;
        line.clear();
    }
    output_writer.seek(io::SeekFrom::Start(header.get_header_size().into())).unwrap();
    for i in 0..hashmap_size {
        output_writer.write_all(&index_map[i as usize].to_be_bytes()).unwrap();
    }
}

fn search(keyword: String, filename: String, column: usize, separator: String) -> bool{
    let is_compressed = filename.ends_with(".gz");
    //Get file reader for original file
    let mut original_file_reader: FileReader = match is_compressed{
        true => FileReader::get_gz_reader(&filename),
        false => FileReader::get_reader(&filename),
    };
    //Get reader for index file
    let mut index_reader = FileReader::get_reader(&format!("{}.index", filename));
    //Read the header size
    let mut buffer = [0; 8];
    index_reader.read_exact(&mut buffer).unwrap();
    let header_size = buffer[0];
    index_reader.seek(0);
    let mut buf: Vec<u8> = vec![0; header_size as usize];
    index_reader.read_exact(&mut buf).unwrap();
    let header = header::Header::from_bytes(buf);
    
    //Initialize variables
    let hashmap_size = header.hashmap_size as u128;
    let hashmap_start = header_size as u64;
    let hash_value = hash_function(&keyword, hashmap_size);
    let hashmap_offset = hashmap_start + (hash_value * HASHMAP_ENTRY_SIZE as u64);
    
    //Read the first block
    index_reader.seek(hashmap_offset);
    let mut buffer = [0; 8];
    index_reader.read_exact(&mut buffer).unwrap();
    //let mut current_block = u64::from_be_bytes(buffer);
    let mut current_index = IndexEntry::from_be_bytes(buffer);

    fn test_file(file_offset: u64, original_file_reader: &mut FileReader, keyword: &String, column: usize, separator: &String) -> bool{
        original_file_reader.seek(file_offset);
        let mut line = String::new();
        original_file_reader.read_line(&mut line).unwrap();
        let mut parts = line.split(separator);
        let value = parts.nth(column).unwrap();
        if value == keyword {
            println!("{}", line);
            return true;
        }
        return false;
    }

    loop{
        match current_index.get_type(){
            IndexEntryType::NULL => {
                println!("Keyword not found");
                return false;
            }
            IndexEntryType::Direct => {
                let file_offset = current_index.get_offset();
                if test_file(file_offset, &mut original_file_reader, &keyword, column, &separator){
                    return true;
                }
                println!("Keyword not found");
                return false;
            }
            IndexEntryType::Indirect => {
                let mut buffer = [0; 16];
                index_reader.seek(current_index.get_offset());
                index_reader.read_exact(&mut buffer).unwrap();
                let file_offset = u64::from_be_bytes(buffer[0..8].try_into().unwrap());
                current_index = IndexEntry::from_be_bytes(buffer[8..16].try_into().unwrap());
                if test_file(file_offset, &mut original_file_reader, &keyword, column, &separator){
                    return true;
                }
            }
        }
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
        assert! (search(format!("prova{}", i), "test.csv".to_string(), 1, ",".to_string()));
    }
    assert! (!search("NOT_EXISTING".to_string(), "test.csv".to_string(), 1, ",".to_string()));
}
fn run_test_compressed(){
    let path = Path::new("test.csv.gz");
    let file = File::create(&path).unwrap();
    let buf_writer = io::BufWriter::new(file);
    let mut writer = BGZFWriter::new(buf_writer, Compression::default());
    for i in 0..100 {
        let string = format!("prova{}", i);
        let _ =writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
    }
    let _ = writer.flush();
    let _ = writer.close();
    index("test.csv.gz".to_string(), 1, 0, ",".to_string());
    for i in 0..100 {
        assert! (search(format!("prova{}", i), "test.csv.gz".to_string(), 1, ",".to_string()));
    }
    assert! (!search("NOT_EXISTING".to_string(), "test.csv.gz".to_string(), 1, ",".to_string()));
}


fn main() {
    run_test_compressed();
    run_test();
    //index("data.csv".to_string(), 0, 0, ",".to_string());
    //search("prova2".to_string(), "data.csv".to_string(), 0, 0, ",".to_string());
}
