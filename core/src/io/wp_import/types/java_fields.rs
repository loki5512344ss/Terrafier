use std::collections::HashMap;

use super::super::java_reader::JvmStream;
use super::java_types::{
    ClassDesc, FieldInfo, JvmValue, PRIM_BOOL, PRIM_BYTE, PRIM_CHAR, PRIM_DOUBLE, PRIM_FLOAT,
    PRIM_INT, PRIM_LONG, PRIM_SHORT, SC_WRITE_METHOD, TC_BLOCKDATA, TC_BLOCKDATALONG,
};

impl JvmStream {
    pub fn read_object_fields(&mut self, cd: &ClassDesc) -> std::io::Result<JvmValue> {
        let mut map = HashMap::new();

        for field in &cd.fields {
            let val = self.read_object_field(field)?;
            map.insert(field.name.clone(), val);
        }

        if (cd.flags & SC_WRITE_METHOD) != 0 {
            self.read_class_annotations()?;
        }

        let super_name = self.find_super_class_name(&cd.name);
        if let Some(super_name) = super_name
            && !super_name.is_empty()
            && super_name != "java.lang.Object"
            && let Some(super_cd) = self.resolve_class_desc(&super_name)
        {
            let super_fields = self.read_object_fields(&super_cd)?;
            if let JvmValue::Object(super_map) = super_fields {
                map.extend(super_map);
            }
        }

        Ok(JvmValue::Object(map))
    }

    pub fn find_super_class_name(&self, _class_name: &str) -> Option<String> {
        None
    }

    pub fn read_object_field(&mut self, field: &FieldInfo) -> std::io::Result<JvmValue> {
        match field.type_code {
            PRIM_BYTE => Ok(JvmValue::Int(self.read_byte()? as i32)),
            PRIM_SHORT => Ok(JvmValue::Int(self.read_u16()? as i16 as i32)),
            PRIM_INT => Ok(JvmValue::Int(self.read_i32()?)),
            PRIM_LONG => Ok(JvmValue::Long(self.read_i64()?)),
            PRIM_FLOAT => {
                let v = self.read_f32()?;
                Ok(JvmValue::Int(v as i32))
            }
            PRIM_DOUBLE => {
                let v = self.read_f64()?;
                Ok(JvmValue::Long(v as i64))
            }
            PRIM_BOOL => Ok(JvmValue::Int(self.read_byte()? as i32)),
            PRIM_CHAR => Ok(JvmValue::Int(self.read_u16()? as i32)),
            b'L' | b'[' => self.read_content(),
            _ => self.read_content(),
        }
    }

    pub fn read_array(&mut self) -> std::io::Result<JvmValue> {
        let class_desc_val = self.read_classdesc()?;
        let class_name = class_desc_val.as_string().unwrap_or_default();
        let length = self.read_i32()? as usize;

        if length == 0 {
            self.push_handle(JvmValue::Skipped);
            return Ok(JvmValue::Skipped);
        }

        let elem_type = class_name.as_bytes().get(1).copied();

        let result = match elem_type {
            Some(PRIM_BYTE) => {
                let bytes = self.read_bytes(length)?;
                let arr: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
                JvmValue::ByteArray(arr)
            }
            Some(PRIM_SHORT) => {
                let mut arr = Vec::with_capacity(length);
                for _ in 0..length {
                    arr.push(self.read_u16()? as i16);
                }
                JvmValue::ShortArray(arr)
            }
            Some(PRIM_INT) => {
                let mut arr = Vec::with_capacity(length);
                for _ in 0..length {
                    arr.push(self.read_i32()?);
                }
                JvmValue::IntArray(arr)
            }
            Some(PRIM_LONG) => {
                for _ in 0..length {
                    let _ = self.read_i64()?;
                }
                JvmValue::LongArray(())
            }
            _ => {
                for _ in 0..length {
                    let _elem = self.read_content()?;
                }
                JvmValue::Skipped
            }
        };

        self.push_handle(result.clone());
        Ok(result)
    }

    pub fn read_hashmap_data(&mut self) -> std::io::Result<JvmValue> {
        let _capacity = self.read_i32()?;
        let _load_factor = self.read_i32()?;
        let _threshold = self.read_i32()?;
        let size = self.read_i32()?;

        let mut map = HashMap::new();
        for _ in 0..size {
            let key = self.read_content()?;
            let value = self.read_content()?;
            let key_str = key
                .as_string()
                .unwrap_or_else(|| format!("__key_{}", map.len()));
            map.insert(key_str, value);
        }

        Ok(JvmValue::Object(map))
    }

    pub fn read_hashset_data(&mut self) -> std::io::Result<JvmValue> {
        let _capacity = self.read_i32()?;
        let _load_factor = self.read_i32()?;
        let _threshold = self.read_i32()?;
        let size = self.read_i32()?;

        let mut map = HashMap::new();
        for i in 0..size {
            let val = self.read_content()?;
            let key = val.as_string().unwrap_or_else(|| format!("__elem_{}", i));
            map.insert(key, val);
        }

        Ok(JvmValue::Object(map))
    }

    pub fn read_arraylist_data(&mut self) -> std::io::Result<JvmValue> {
        let size = self.read_i32()?;

        let mut map = HashMap::new();
        for i in 0..size {
            let val = self.read_content()?;
            let key = val.as_string().unwrap_or_else(|| format!("__elem_{}", i));
            map.insert(key, val);
        }

        Ok(JvmValue::Object(map))
    }

    pub fn resolve_class_desc(&self, name: &str) -> Option<ClassDesc> {
        for cd in &self.class_descs {
            if cd.name == name {
                return Some(cd.clone());
            }
        }
        None
    }

    pub fn skip_object_data(&mut self, _class_name: &str) -> std::io::Result<()> {
        let saved = self.pos;
        match self.read_class_annotations() {
            Ok(()) => Ok(()),
            Err(_) => {
                self.pos = saved;
                if self.pos < self.data.len() {
                    let tc = match self.peek_byte() {
                        Ok(t) => t,
                        Err(_) => return Ok(()),
                    };
                    if tc == TC_BLOCKDATA || tc == TC_BLOCKDATALONG {
                        return self.read_class_annotations();
                    }
                    let _remaining = self.data.len() - self.pos;
                    self.pos = self.data.len();
                    Ok(())
                } else {
                    Ok(())
                }
            }
        }
    }
}
