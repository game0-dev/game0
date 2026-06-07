use crate::AssetId128;

#[derive(Clone, Debug, Default)]
pub struct EncodeBuffer {
    bytes: Vec<u8>,
}

impl EncodeBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_u16_le(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i32_le(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64_le(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_f32_le(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn write_asset_id128(&mut self, id: AssetId128) {
        self.bytes.extend_from_slice(&id.0);
    }

    pub fn pad_to_align(&mut self, align: usize) {
        if align <= 1 {
            return;
        }
        let rem = self.bytes.len() % align;
        if rem != 0 {
            self.bytes.resize(self.bytes.len() + align - rem, 0);
        }
    }
}
