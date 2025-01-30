
const HEADER_SIZE: [u8; 1] = [10];

pub struct Header{
    pub version: u8,
    pub hashmap_size: u64
}
impl Header{
    pub fn new(version: u8, hashmap_size: u64) -> Header{
        return Header{version, hashmap_size};
    }
    pub fn get_header_size(&self) -> u8{
        HEADER_SIZE[self.version as usize]
    }
    pub fn to_bytes(&self) -> Vec<u8>{
        let mut bytes: Vec<u8> = Vec::with_capacity(self.get_header_size() as usize);
        bytes.push(self.get_header_size());
        bytes.push(self.version);
        let _ = &self.hashmap_size.to_be_bytes().map(|x| bytes.push(x));
        assert!(bytes.len() == self.get_header_size() as usize);
        return bytes;
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Header{
        let _ = bytes[0];
        let version = bytes[1];
        let hashmap_size = u64::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9]]);
        return Header{version, hashmap_size};
    }
}