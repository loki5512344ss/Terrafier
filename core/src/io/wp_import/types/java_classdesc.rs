use std::collections::HashMap;

use super::super::java_reader::JvmStream;
use super::java_types::{
    BASE_WIRE_HANDLE, ClassDesc, FieldInfo, JvmValue, TC_ARRAY, TC_BLOCKDATA, TC_BLOCKDATALONG,
    TC_CLASS, TC_CLASSDESC, TC_ENDBLOCKDATA, TC_ENUM, TC_LONGSTRING, TC_NULL, TC_OBJECT,
    TC_PROXYCLASSDESC, TC_REFERENCE, TC_RESET, TC_STRING,
};

impl JvmStream {
    pub fn read_classdesc(&mut self) -> std::io::Result<JvmValue> {
        let tc = self.peek_byte()?;
        match tc {
            TC_CLASSDESC => self.read_classdesc_body(),
            TC_REFERENCE => {
                self.read_byte()?;
                let handle = self.read_u32_handle();
                let idx = (handle - BASE_WIRE_HANDLE) as usize;
                if idx < self.handles.len() {
                    Ok(self.handles[idx].clone())
                } else {
                    Ok(JvmValue::Null)
                }
            }
            TC_NULL => {
                self.read_byte()?;
                Ok(JvmValue::Null)
            }
            TC_PROXYCLASSDESC => {
                self.read_byte()?;
                let _iface_count = self.read_i32()?;
                for _ in 0.._iface_count {
                    let _iface = self.read_utf()?;
                }
                self.read_class_annotations()?;
                let _super = self.read_classdesc()?;
                self.push_handle(JvmValue::Skipped);
                Ok(JvmValue::Skipped)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected classdesc token, got 0x{:02x}", tc),
            )),
        }
    }

    pub fn read_classdesc_body(&mut self) -> std::io::Result<JvmValue> {
        self.read_byte()?;
        let class_name = self.read_utf()?;
        let _serial_version_uid = self.read_i64()?;
        let desc_flags = self.read_byte()?;
        let field_count = self.read_u16()?;

        let mut fields = Vec::new();
        for _ in 0..field_count {
            let type_code = self.read_byte()?;
            let name = self.read_utf()?;
            if type_code == b'[' || type_code == b'L' {
                let tc = self.read_byte()?;
                match tc {
                    TC_STRING => {
                        self.read_utf()?;
                    }
                    TC_REFERENCE => {
                        let handle = self.read_u32_handle();
                        let idx = (handle - BASE_WIRE_HANDLE) as usize;
                        if idx < self.handles.len() {
                            let _ = self.handles[idx].as_string();
                        }
                    }
                    TC_LONGSTRING => {
                        self.read_long_utf()?;
                    }
                    _ => {}
                }
            }
            fields.push(FieldInfo { name, type_code });
        }

        let cd = ClassDesc {
            name: class_name.clone(),
            flags: desc_flags,
            fields,
        };
        self.class_descs.push(cd.clone());
        self.push_handle(JvmValue::ClassDesc(cd.clone()));
        self.read_class_annotations()?;
        let _super = self.read_classdesc()?;

        Ok(JvmValue::ClassDesc(cd))
    }

    pub fn read_class_annotations(&mut self) -> std::io::Result<()> {
        loop {
            let tc = self.read_byte()?;
            match tc {
                TC_ENDBLOCKDATA => return Ok(()),
                TC_BLOCKDATA => {
                    let len = self.read_byte()? as usize;
                    self.pos += len;
                }
                TC_BLOCKDATALONG => {
                    let len = self.read_i32()? as usize;
                    self.pos += len;
                }
                TC_OBJECT | TC_ARRAY | TC_STRING | TC_LONGSTRING | TC_ENUM => {
                    self.pos -= 1;
                    let _val = self.read_content()?;
                }
                TC_REFERENCE => {
                    let _handle = self.read_u32_handle();
                }
                TC_CLASSDESC | TC_PROXYCLASSDESC => {
                    self.pos -= 1;
                    let _desc = self.read_classdesc()?;
                }
                TC_NULL => {}
                TC_CLASS => {
                    let _desc = self.read_classdesc()?;
                }
                TC_RESET => {
                    self.handles.clear();
                }
                other => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unexpected token in class annotations: 0x{:02x}", other),
                    ));
                }
            }
        }
    }

    pub fn read_object(&mut self) -> std::io::Result<JvmValue> {
        let class_desc_val = self.read_classdesc()?;
        let cd = match &class_desc_val {
            JvmValue::ClassDesc(cd) => cd.clone(),
            JvmValue::Null => {
                return Ok(JvmValue::Object(HashMap::new()));
            }
            _ => {
                let name = class_desc_val.as_string().unwrap_or_default();
                let cd = self.resolve_class_desc(&name);
                match cd {
                    Some(cd) => cd,
                    None => {
                        let _handle = self.push_handle(JvmValue::Skipped);
                        self.skip_object_data(&name)?;
                        return Ok(JvmValue::Skipped);
                    }
                }
            }
        };

        let handle = self.push_handle(JvmValue::Skipped);

        let obj = match cd.name.as_str() {
            "java.util.HashMap" | "java.util.LinkedHashMap" => self.read_hashmap_data()?,
            "java.util.HashSet" | "java.util.LinkedHashSet" => self.read_hashset_data()?,
            "java.util.ArrayList" => self.read_arraylist_data()?,
            "java.lang.Integer" => {
                let val = self.read_i32()?;
                self.handles[(handle - BASE_WIRE_HANDLE) as usize] = JvmValue::Int(val);
                return Ok(JvmValue::Int(val));
            }
            "java.lang.Long" => {
                let val = self.read_i64()?;
                self.handles[(handle - BASE_WIRE_HANDLE) as usize] = JvmValue::Long(val);
                return Ok(JvmValue::Long(val));
            }
            "java.lang.String" => {
                let val = self.read_content()?;
                self.handles[(handle - BASE_WIRE_HANDLE) as usize] = val.clone();
                return Ok(val);
            }
            _ => self.read_object_fields(&cd)?,
        };

        self.handles[(handle - BASE_WIRE_HANDLE) as usize] = obj.clone();
        Ok(obj)
    }
}
