
const HEADER_SIZE: [u8; 1] = [13];

pub struct Header{
    pub version: u8,
    pub hashmap_size: u64,
    pub index_type: u8, //0: tabular, 1,2 fasta with id and seq, 3,4 fastq with id and seq
    pub separator: u8,
    pub column: u8
}
impl Header{
    pub fn new(version: u8, hashmap_size: u64, index_type: u8, separator: u8, column: u8) -> Header{
        return Header{version, hashmap_size, index_type: index_type, separator, column};
    }
    pub fn get_header_size(&self) -> u8{
        HEADER_SIZE[self.version as usize]
    }
    pub fn to_bytes(&self) -> Vec<u8>{
        let mut bytes: Vec<u8> = Vec::with_capacity(self.get_header_size() as usize);
        bytes.push(self.get_header_size());
        bytes.push(self.version);
        let _ = &self.hashmap_size.to_be_bytes().map(|x| bytes.push(x));
        bytes.push(self.index_type);
        bytes.push(self.separator);
        bytes.push(self.column);
        assert!(bytes.len() == self.get_header_size() as usize);
        return bytes;
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Header{
        let _ = bytes[0];
        let version = bytes[1];
        let hashmap_size = u64::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9]]);
        let index_type = bytes[10];
        let separator = bytes[11];
        let column = bytes[12];
        return Header{version, hashmap_size, index_type, separator, column};
    }
}