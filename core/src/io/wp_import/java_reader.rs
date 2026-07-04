use super::types::{
    BASE_WIRE_HANDLE, ClassDesc, JvmValue, TC_ARRAY, TC_BLOCKDATA, TC_BLOCKDATALONG, TC_CLASS,
    TC_ENUM, TC_LONGSTRING, TC_NULL, TC_OBJECT, TC_REFERENCE, TC_RESET, TC_STRING,
};

pub struct JvmStream {
    pub data: Vec<u8>,
    pub pos: usize,
    pub handles: Vec<JvmValue>,
    pub class_descs: Vec<ClassDesc>,
}

impl JvmStream {
    pub fn new(data: Vec<u8>) -> Self {
        JvmStream {
            data,
            pos: 0,
            handles: Vec::new(),
            class_descs: Vec::new(),
        }
    }

    pub fn read_byte(&mut self) -> std::io::Result<u8> {
        if self.pos >= self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected end of stream",
            ));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    pub fn read_bytes(&mut self, n: usize) -> std::io::Result<Vec<u8>> {
        if self.pos + n > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected end of stream",
            ));
        }
        let slice = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(slice)
    }

    pub fn peek_byte(&self) -> std::io::Result<u8> {
        if self.pos >= self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected end of stream",
            ));
        }
        Ok(self.data[self.pos])
    }

    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        let hi = self.read_byte()? as u16;
        let lo = self.read_byte()? as u16;
        Ok((hi << 8) | lo)
    }

    pub fn read_i32(&mut self) -> std::io::Result<i32> {
        let b1 = self.read_byte()? as i32;
        let b2 = self.read_byte()? as i32;
        let b3 = self.read_byte()? as i32;
        let b4 = self.read_byte()? as i32;
        Ok((b1 << 24) | (b2 << 16) | (b3 << 8) | b4)
    }

    pub fn read_i64(&mut self) -> std::io::Result<i64> {
        let b1 = self.read_byte()? as i64;
        let b2 = self.read_byte()? as i64;
        let b3 = self.read_byte()? as i64;
        let b4 = self.read_byte()? as i64;
        let b5 = self.read_byte()? as i64;
        let b6 = self.read_byte()? as i64;
        let b7 = self.read_byte()? as i64;
        let b8 = self.read_byte()? as i64;
        Ok((b1 << 56)
            | (b2 << 48)
            | (b3 << 40)
            | (b4 << 32)
            | (b5 << 24)
            | (b6 << 16)
            | (b7 << 8)
            | b8)
    }

    pub fn read_f32(&mut self) -> std::io::Result<f32> {
        Ok(f32::from_bits(self.read_i32()? as u32))
    }

    pub fn read_f64(&mut self) -> std::io::Result<f64> {
        Ok(f64::from_bits(self.read_i64()? as u64))
    }

    pub fn read_utf(&mut self) -> std::io::Result<String> {
        let len = self.read_u16()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub fn read_long_utf(&mut self) -> std::io::Result<String> {
        let len = self.read_i64()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub fn read_content(&mut self) -> std::io::Result<JvmValue> {
        loop {
            if self.pos >= self.data.len() {
                return Ok(JvmValue::Null);
            }
            let tc = self.read_byte()?;
            match tc {
                TC_OBJECT => return self.read_object(),
                TC_ARRAY => return self.read_array(),
                TC_STRING => {
                    let s = self.read_utf()?;
                    self.push_handle(JvmValue::String(s.clone()));
                    return Ok(JvmValue::String(s));
                }
                TC_LONGSTRING => {
                    let s = self.read_long_utf()?;
                    self.push_handle(JvmValue::String(s.clone()));
                    return Ok(JvmValue::String(s));
                }
                TC_REFERENCE => {
                    let handle = self.read_u32_handle();
                    let idx = (handle - BASE_WIRE_HANDLE) as usize;
                    if idx < self.handles.len() {
                        return Ok(self.handles[idx].clone());
                    }
                    return Ok(JvmValue::Null);
                }
                TC_NULL => return Ok(JvmValue::Null),
                TC_CLASS => {
                    let _desc = self.read_classdesc()?;
                    return Ok(JvmValue::Skipped);
                }
                TC_ENUM => {
                    let _desc = self.read_classdesc()?;
                    let _ordinal = self.read_i32()?;
                    return Ok(JvmValue::Skipped);
                }
                TC_BLOCKDATA => {
                    let len = self.read_byte()? as usize;
                    self.pos += len;
                }
                TC_BLOCKDATALONG => {
                    let len = self.read_i32()? as usize;
                    self.pos += len;
                }
                TC_RESET => {
                    self.handles.clear();
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unexpected content token: 0x{:02x}", tc),
                    ));
                }
            }
        }
    }

    pub fn push_handle(&mut self, val: JvmValue) -> u32 {
        let idx = self.handles.len();
        self.handles.push(val);
        BASE_WIRE_HANDLE + idx as u32
    }

    pub fn read_u32_handle(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.data[self.pos..self.pos + 4]);
        self.pos += 4;
        u32::from_be_bytes(buf)
    }
}
