mod file_reader;
mod file_writer;
mod header;
mod index_structure;

use std::fs::File;
use std::io::{self};
use std::path::Path;
use std::io::Write;
use std::vec;
use bgzip::{BGZFWriter, Compression};
use clap::Parser;
use stable_hash::fast_stable_hash;
use file_reader::{FileReader, InputReader};
use index_structure::{IndexStructure, IndexEntry, IndexEntryType, HASHMAP_ENTRY_SIZE};


const CURRENT_VERSION: u8 = 0;


fn hash_function(value: &str, hashmap_size: u128) -> u64 {
    let r = fast_stable_hash(&value) % hashmap_size;
    return r as u64;
}




fn index(filename: String, column: usize, mut hashmap_size: u128, separator: String, in_memory_map_size: u64) {
    let is_compressed = filename.ends_with(".gz");
    //Create reader
    let mut file_input_reader: FileReader = match is_compressed{
        true => FileReader::get_gz_reader(&filename),
        false => FileReader::get_reader(&filename),
    };
    let mut input_reader: InputReader = InputReader::new(file_input_reader);
    //If unspecified, set hashmap_size to number of lines
    if hashmap_size == 0 {
        hashmap_size = input_reader.num_entries() as u128;
    }

    //Create header object
    let header = header::Header::new(CURRENT_VERSION, hashmap_size as u64);
    //Create the index structure
    let mut index_structure = IndexStructure::new(filename, header, in_memory_map_size);
    
    let mut line = String::new();
    loop{
        loop {
            let offset = input_reader.get_entry(&mut line);
            if offset == 0xFFFFFFFFFFFFFFFF {
                break;
            }
            let mut parts = line.split(&separator);
            let value = parts.nth(column).unwrap();
            let hash = hash_function(value, hashmap_size);
            index_structure.add_entry(hash, offset as u64);
            line.clear();
        }
        if !index_structure.next(){
            break;
        }
        input_reader.reset();
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
                if current_index.get_type() == IndexEntryType::Direct{
                    if test_file(current_index.get_offset(), &mut original_file_reader, &keyword, column, &separator){
                        return true;
                    }
                    println!("Keyword not found");
                    return false;
                }
            }
        }
    }
}

fn run_test(in_memory_map_size: u64){
    let path = Path::new("test.csv");
    let file = File::create(&path).unwrap();
    let mut writer = io::BufWriter::new(file);
    for i in 0..100 {
        let string = format!("prova{}", i);
        let _ = writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
        //writer.write_line();
    }
    let _ = writer.flush();
    index("test.csv".to_string(), 1, 0, ",".to_string(), in_memory_map_size);
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
    index("test.csv.gz".to_string(), 1, 0, ",".to_string(),1000);
    for i in 0..100 {
        assert! (search(format!("prova{}", i), "test.csv.gz".to_string(), 1, ",".to_string()));
    }
    assert! (!search("NOT_EXISTING".to_string(), "test.csv.gz".to_string(), 1, ",".to_string()));
}

mod command_line_tool;
use command_line_tool::{Cli, Commands};
fn main_() {
    //index("data.csv".to_string(), 0, 0, ",".to_string());
    //search("prova2".to_string(), "data.csv".to_string(), 0, 0, ",".to_string());
    let cli = Cli::parse();
    match cli.command {
        Commands::Index { filename, column, separator, hashmap_size, in_memory_map_size} => {
            index(filename, column, hashmap_size, separator, in_memory_map_size);
        }
        Commands::Search { filename, keyword, column, separator, print_duplicates } => {
            search(keyword, filename, column, separator);
        }
        Commands::Test{} => { 
            run_test_compressed();
            run_test(2000000000);
         }
    }
}
fn main(){
    run_test_compressed();
    run_test(10000);
    run_test(5);
}
