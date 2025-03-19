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
use file_reader::{FileReader, GzFileReader, InputReader, StandardFileReader, TabularInputReader};
use index_structure::{IndexStructure, IndexEntry, IndexEntryType, HASHMAP_ENTRY_SIZE};


const CURRENT_VERSION: u8 = 0;


fn hash_function(value: &str, hashmap_size: u128) -> u64 {
    let r = fast_stable_hash(&value) % hashmap_size;
    return r as u64;
}

fn index(input_reader: &mut impl InputReader, filename: String, mut hashmap_size: u128, in_memory_map_size: u64) {
    //If unspecified, set hashmap_size to number of lines
    if hashmap_size == 0 {
        hashmap_size = input_reader.num_entries() as u128;
    }

    //Create header object
    let header = header::Header::new(CURRENT_VERSION, hashmap_size as u64);
    //Create the index structure
    let mut index_structure = IndexStructure::new(filename, header, in_memory_map_size);
    hashmap_size = index_structure.header.hashmap_size as u128;
    let mut line = String::new();
    loop{
        loop {
            let offset = input_reader.get_entry(&mut line);
            if offset == 0xFFFFFFFFFFFFFFFF {
                break;
            }
            let hash = hash_function(&line, hashmap_size);
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
    //Get file reader for original file
    let original_file_reader: &mut dyn FileReader = match filename.ends_with(".gz"){
        true => &mut GzFileReader::new(&filename),
        false => &mut StandardFileReader::new(&filename),
    };
    let mut input_reader: TabularInputReader = TabularInputReader::new(
        original_file_reader, &separator, column
    );
    //Get reader for index file
    let mut index_reader = StandardFileReader::new(&format!("{}.index", filename));
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
    let mut test_buffer: String = String::new();
    loop{
        match current_index.get_type(){
            IndexEntryType::NULL => {
                println!("Keyword not found");
                return false;
            }
            IndexEntryType::Direct => {
                let file_offset = current_index.get_offset();
                if input_reader.test_and_return_entry(file_offset, &keyword, &mut test_buffer){
                    println!("{}", test_buffer);
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
                if input_reader.test_and_return_entry(file_offset, &keyword, &mut test_buffer){
                    println!("{}", test_buffer);
                    return true;
                }
                test_buffer.clear();
                if current_index.get_type() == IndexEntryType::Direct{
                    if input_reader.test_and_return_entry(current_index.get_offset(), &keyword, &mut test_buffer){
                        println!("{}", test_buffer);
                        return true;
                    }
                    println!("Keyword not found");
                    return false;
                }
            }
        }
    }
}

fn index_tabular(filename: String, column: usize, separator: String, hashmap_size: u128, in_memory_map_size: u64){
    let file_input_reader: &mut dyn FileReader = match filename.ends_with(".gz"){
        true => &mut GzFileReader::new(&filename),
        false => &mut StandardFileReader::new(&filename),
    };
    let mut input_reader: TabularInputReader = TabularInputReader::new(
        file_input_reader, &separator, column
    );
    index(&mut input_reader, filename, hashmap_size, in_memory_map_size);
}

mod command_line_tool;
use command_line_tool::{Cli, Commands};
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::IndexTabular { filename, column, separator, hashmap_size, in_memory_map_size} => {
            index_tabular(filename, column, separator, hashmap_size, in_memory_map_size);
        }
        Commands::IndexFasta { filename, by_sequence, hashmap_size, in_memory_map_size } => {
            //
        }
        Commands::IndexFastq { filename, by_sequence, hashmap_size, in_memory_map_size } => {
            //
        }
        Commands::Search { filename, keyword, column, separator, print_duplicates } => {
            search(keyword, filename, column, separator);
        }
        Commands::Test{} => { 
            test();
         }
    }
}

const TEST_LEN: u32 = 100;

fn run_test(in_memory_map_size: u64){
    let path = Path::new("test.csv");
    let file = File::create(&path).unwrap();
    let mut writer = io::BufWriter::new(file);
    for i in 0..TEST_LEN {
        let string = format!("prova{}", i);
        let _ = writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
        //writer.write_line();
    }
    let _ = writer.flush();
    index_tabular("test.csv".to_string(), 1, ",".to_string(), 0, in_memory_map_size);
    for i in 0..TEST_LEN {
        assert! (search(format!("prova{}", i), "test.csv".to_string(), 1, ",".to_string()));
    }
    assert! (!search("NOT_EXISTING".to_string(), "test.csv".to_string(), 1, ",".to_string()));
}
fn run_test_compressed(){
    let path = Path::new("test.csv.gz");
    let file = File::create(&path).unwrap();
    let buf_writer = io::BufWriter::new(file);
    let mut writer = BGZFWriter::new(buf_writer, Compression::default());
    for i in 0..TEST_LEN {
        let string = format!("prova{}", i);
        let _ =writer.write_all(format!("1,{},0,0,0,eruheigrneiugrheriuhg,ergbneirgbeiugberiugberiuhg\n", string).as_bytes());
    }
    let _ = writer.flush();
    let _ = writer.close();
    index_tabular("test.csv.gz".to_string(), 1, ",".to_string(), 0, 1000);
    for i in 0..TEST_LEN {
        assert! (search(format!("prova{}", i), "test.csv.gz".to_string(), 1, ",".to_string()));
    }
    assert! (!search("NOT_EXISTING".to_string(), "test.csv.gz".to_string(), 1, ",".to_string()));
}
fn test(){
    run_test(10000);
    run_test_compressed();
    run_test(6);
}
