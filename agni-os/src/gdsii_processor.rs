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

    pub fn next_record(&mut self) -> Result<Option<(u8, Vec<u8>)>, String> {
        let pos = self.cursor.position();

        if self.total_len - pos < 4 {
            if self.total_len - pos == 0 { return Ok(None); }
            return Err("GDSII_TRUNCATED: Incomplete header at EOF".into());
        }

        let record_len = self.cursor.read_u16::<BigEndian>().map_err(|_| "IO Error")?;

        if record_len < 4 {
            return Err(format!("GDSII_CORRUPT: Invalid record length {}", record_len));
        }

        let remaining = self.total_len - pos;
        if (record_len as u64) > remaining {
            return Err(format!("GDSII_BUFFER_OVERREAD: Record claims {} bytes, only {} remain", record_len, remaining));
        }

        let rec_type = self.cursor.read_u8().map_err(|_| "GDSII_EOF: Failed to read record type")?;
        let _data_type = self.cursor.read_u8().map_err(|_| "GDSII_EOF: Failed to read data type")?;

        let payload_len = (record_len - 4) as usize;
        let mut payload = vec![0u8; payload_len];
        self.cursor.read_exact(&mut payload).map_err(|_| "Payload read failed")?;

        Ok(Some((rec_type, payload)))
    }

    pub fn next_polygon(&mut self) -> Result<Option<Vec<(f64, f64)>>, String> {
        let mut points = Vec::new();
        let mut in_boundary = false;

        loop {
            match self.next_record()? {
                Some((rec_type, payload)) => {
                    // BOUNDARY = 0x08
                    if rec_type == 0x08 {
                        in_boundary = true;
                    }
                    // XY = 0x10
                    else if rec_type == 0x10 && in_boundary {
                        // Parse XY: List of 4-byte signed integers (Big Endian)
                        if payload.len() % 8 != 0 {
                            return Err("GDSII_XY: Malformed coordinate list".into());
                        }
                        let count = payload.len() / 8;
                        let mut rdr = Cursor::new(payload);
                        for _ in 0..count {
                            let x = rdr.read_i32::<BigEndian>().unwrap_or(0) as f64 / 1000.0; // Scale? Assumed nm/um
                            let y = rdr.read_i32::<BigEndian>().unwrap_or(0) as f64 / 1000.0;
                            points.push((x, y));
                        }
                    }
                    // ENDEL = 0x11
                    else if rec_type == 0x11 && in_boundary {
                        return Ok(Some(points));
                    }
                },
                None => return Ok(None),
            }
        }
    }
}
