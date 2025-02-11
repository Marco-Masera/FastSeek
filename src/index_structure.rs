use crate::file_writer;
use crate::header;
use file_writer::FileWriter;
use header::Header;
use std::io::{self};
use std::cmp::min;

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
    blocks_buffer_used: usize,
    block_buffer_output_offset: u64,
    in_memory_map_size: u64,
    margin_l: u64,
    margin_h: u64
}

impl IndexStructure{
    pub fn new(filename: String, header: Header, mut in_memory_map_size: u64) -> IndexStructure{
        let hashmap_size = header.hashmap_size;
        in_memory_map_size = min(in_memory_map_size, hashmap_size);

        let mut structure =  IndexStructure{
            file_writer: FileWriter::get_writer(format!("{}.index", filename)),
            header,
            index_map: vec![],
            blocks_buffer: [0; BLOCK_BUFFER_SIZE],
            block_first_free:0,
            blocks_buffer_used:0,
            block_buffer_output_offset:0,
            in_memory_map_size:in_memory_map_size,
            margin_l:0,
            margin_h:min(in_memory_map_size, hashmap_size)
        };

        //Write header to index file
        structure.file_writer.write_all(&(structure.header).to_bytes()).unwrap();

        //In-memory structure for the hashmap (TODO: might have to do on-disk)
        structure.index_map = (0..min(hashmap_size,in_memory_map_size)).map(|_| IndexEntry::new_null()).collect::<Vec<_>>();

        //Keep indexes where to write the blocks
        let block_starting_address: u64 = (hashmap_size as u64 * HASHMAP_ENTRY_SIZE as u64) + (structure.header.get_header_size() as u64);
        structure.block_first_free = block_starting_address;
        structure.block_buffer_output_offset = block_starting_address;
        //Set buffer for block part of the index
        structure.blocks_buffer_used = 0;
        let _ = structure.file_writer.seek(io::SeekFrom::Start(block_starting_address));

        return structure;
    }

    pub fn add_entry(&mut self, mut hash: u64, file_offset: u64){
        if hash<self.margin_l || hash>= self.margin_h{
            return;
        }
        hash -= self.margin_l;

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

    pub fn next(&mut self) -> bool{
        //Write hashmap from memory to file
        self.file_writer.seek(io::SeekFrom::Start(
            self.header.get_header_size() as u64 + (self.margin_l*(HASHMAP_ENTRY_SIZE as u64))
        )).unwrap();
        let buf_capacity = 1080*8;
        let mut buffer: Vec<u8> = Vec::with_capacity(buf_capacity);
        //Using unsafe for direct pointer access and faster runtime
        //"Cowards die many times before their deaths; the valiant never taste of death but once" -William Shakespeare
        for chunk in self.index_map.chunks(buf_capacity/8){
            let bytes_needed = chunk.len() * 8;
            unsafe {
                buffer.clear();
                buffer.set_len(bytes_needed);
                let dest = buffer.as_mut_ptr();
                for (i, item) in chunk.iter().enumerate() {
                    let bytes = item.to_be_bytes();
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        dest.add(i * 8),
                        8
                    );
                }
            }
            self.file_writer.write_all(&buffer[..bytes_needed]).unwrap();
        }

        self.margin_l = self.margin_h;
        self.margin_h = min(self.margin_h+self.in_memory_map_size, self.header.hashmap_size);
        let keep_on = self.margin_l < self.header.hashmap_size-1;
        if keep_on{
            self.index_map = (0..self.in_memory_map_size).map(|_| IndexEntry::new_null()).collect::<Vec<_>>();
        }
        let _ = self.file_writer.seek(io::SeekFrom::Start(self.block_buffer_output_offset));
        if self.blocks_buffer_used > 0 {
            self.flush_block_buffer(self.blocks_buffer_used);
        }
        println!("Iteration");
        return keep_on
    }

    pub fn flush_block_buffer(&mut self, to: usize){
        let _ = self.file_writer.write_all(&self.blocks_buffer[..to]);
        self.blocks_buffer_used = 0;
        self.block_buffer_output_offset += to as u64;
    }

}