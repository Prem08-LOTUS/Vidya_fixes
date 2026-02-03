// agni-workspace/agni-os/src/gdsii_processor.rs
use std::io::{Read, Cursor};
use byteorder::{BigEndian, ReadBytesExt};

pub struct GdsiiStreamingParser {
    cursor: Cursor<Vec<u8>>,
    total_len: u64,
}

impl GdsiiStreamingParser {
    pub fn new(data: Vec<u8>) -> Self {
        let len = data.len() as u64;
        Self {
            cursor: Cursor::new(data),
            total_len: len,
        }
    }

    pub fn next_record(&mut self) -> Result<Option<(u16, Vec<u8>)>, String> {
        let pos = self.cursor.position();

        // 1. Check for EOF header space
        if self.total_len - pos < 4 {
            if self.total_len - pos == 0 { return Ok(None); } // Clean EOF
            return Err("GDSII_TRUNCATED: Incomplete header at EOF".into());
        }

        // 2. Peek Length
        let record_len = self.cursor.read_u16::<BigEndian>().map_err(|_| "IO Error")?;

        // 3. Security Check: Minimum Record Size
        // GDSII spec: Min length is 4 (2 bytes len + 1 byte rec_type + 1 byte data_type)
        if record_len < 4 {
            return Err(format!("GDSII_CORRUPT: Invalid record length {}", record_len));
        }

        // 4. Security Check: Bounds Validation (The "Vandal" Fix)
        // Does the stated length exceed the remaining file size?
        let remaining = self.total_len - pos;
        if (record_len as u64) > remaining {
            return Err(format!(
                "GDSII_BUFFER_OVERREAD: Record claims {} bytes, only {} remain",
                record_len, remaining
            ));
        }

        // 5. Read Payload
        let _rec_type = self.cursor.read_u8().unwrap();
        let _data_type = self.cursor.read_u8().unwrap();

        let payload_len = (record_len - 4) as usize;
        let mut payload = vec![0u8; payload_len];
        self.cursor.read_exact(&mut payload).map_err(|_| "Payload read failed")?;

        Ok(Some((record_len, payload)))
    }
}
