use crate::file_writer;
use crate::header;
use file_writer::FileWriter;
use header::Header;
use std::io::{self};

pub const HASHMAP_ENTRY_SIZE: u8 = 8;
const BLOCK_BUFFER_SIZE:usize = 1024*50*8;

#[derive(PartialEq, Eq)]
pub enum IndexEntryType{
    Direct,
    Indirect,
    NULL
}
pub struct IndexEntry{
    index: u64
}
impl IndexEntry{
    pub fn get_type(&self) -> IndexEntryType{
        if self.index == u64::MAX {
            return IndexEntryType::NULL;
        }
        if self.index & 0x8000000000000000 == 0{
            return IndexEntryType::Direct;
        }
        return IndexEntryType::Indirect;
    }
    pub fn get_offset(&self) -> u64{
        return self.index & 0x7FFFFFFFFFFFFFFF;
    }
    pub fn new_direct(offset: u64) -> IndexEntry{
        return IndexEntry{index: offset};
    }
    pub fn new_indirect(offset: u64) -> IndexEntry{
        return IndexEntry{index: offset | 0x8000000000000000};
    }
    pub fn new_null() -> IndexEntry{
        return IndexEntry{index: u64::MAX};
    }
    pub fn to_be_bytes(&self) -> [u8; 8]{
        return self.index.to_be_bytes();
    }
    pub fn from_be_bytes(bytes: [u8; 8]) -> IndexEntry{
        return IndexEntry{index: u64::from_be_bytes(bytes)};
    }
}


pub struct IndexStructure{
    file_writer: FileWriter,
    header: Header,
    index_map: Vec<IndexEntry>,
    blocks_buffer: [u8; BLOCK_BUFFER_SIZE],
    block_first_free: u64,
    blocks_buffer_used: usize
}

impl IndexStructure{
    pub fn new(filename: String, header: Header) -> IndexStructure{
        let mut structure =  IndexStructure{
            file_writer: FileWriter::get_writer(format!("{}.index", filename)),
            header,
            index_map: vec![],
            blocks_buffer: [0; BLOCK_BUFFER_SIZE],
            block_first_free:0,
            blocks_buffer_used:0
        };

        let hashmap_size = structure.header.hashmap_size;
        //Write header to index file
        structure.file_writer.write_all(&(structure.header).to_bytes()).unwrap();
        //Write empty hashmap
        const CHUNK_SIZE: usize = 1024*8;
        let total_bytes = hashmap_size as usize * (HASHMAP_ENTRY_SIZE as usize);
        let mut remaining = total_bytes;
        let chunk = vec![255; CHUNK_SIZE];
        while remaining > 0 {
            let to_write = remaining.min(CHUNK_SIZE);
            structure.file_writer.write_all(&chunk[..to_write]).unwrap();
            remaining -= to_write;
        }

        //In-memory structure for the hashmap (TODO: might have to do on-disk)
        structure.index_map = (0..hashmap_size).map(|_| IndexEntry::new_null()).collect::<Vec<_>>();

        //Keep indexes where to write the blocks
        let block_starting_address: u64 = (hashmap_size as u64 * HASHMAP_ENTRY_SIZE as u64) + (structure.header.get_header_size() as u64);
        structure.block_first_free = block_starting_address;
        //Set buffer for block part of the index
        structure.blocks_buffer_used = 0;
        let _ = structure.file_writer.seek(io::SeekFrom::Start(block_starting_address));

        return structure;
    }

    pub fn add_entry(&mut self, hash: u64, file_offset: u64){
        let current_index = &self.index_map[hash as usize];
        match current_index.get_type(){
            IndexEntryType::NULL => {self.index_map[hash as usize] = IndexEntry::new_direct(file_offset);}
            IndexEntryType::Indirect => {
                self.blocks_buffer[self.blocks_buffer_used..self.blocks_buffer_used+8].copy_from_slice(&file_offset.to_be_bytes());
                self.blocks_buffer[self.blocks_buffer_used+8..self.blocks_buffer_used+16].copy_from_slice(&current_index.index.to_be_bytes());
                self.index_map[hash as usize] = IndexEntry::new_indirect(self.block_first_free);
                self.block_first_free += 16;
                self.blocks_buffer_used += 16;
            }
            IndexEntryType::Direct => {
                self.blocks_buffer[self.blocks_buffer_used..self.blocks_buffer_used+8].copy_from_slice(&current_index.get_offset().to_be_bytes());
                self.blocks_buffer[self.blocks_buffer_used+8..self.blocks_buffer_used+16].copy_from_slice(&IndexEntry::new_direct(file_offset).index.to_be_bytes());
                self.index_map[hash as usize] = IndexEntry::new_indirect(self.block_first_free);
                self.block_first_free += 16;
                self.blocks_buffer_used += 16;
            }
        }
        if self.blocks_buffer_used == BLOCK_BUFFER_SIZE{
            self.flush_block_buffer(BLOCK_BUFFER_SIZE);
        }
    }

    pub fn flush_block_buffer(&mut self, to: usize){
        let _ = self.file_writer.write_all(&self.blocks_buffer[..to]);
        self.blocks_buffer_used = 0;
    }

    pub fn finalize(&mut self){
        if self.blocks_buffer_used > 0 {
            self.flush_block_buffer(self.blocks_buffer_used);
        }
        self.file_writer.seek(io::SeekFrom::Start(self.header.get_header_size().into())).unwrap();
        for i in 0..self.header.hashmap_size {
            self.file_writer.write_all(&self.index_map[i as usize].to_be_bytes()).unwrap();
        }
    }
}