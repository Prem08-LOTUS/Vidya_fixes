// agni-workspace/agni-os/src/io/grbl_reader.rs
use tracing::{warn, error};

/// A robust, byte-oriented buffer for parsing GRBL/Marlin status lines.
///
/// PHYSICAL GUARANTEES:
/// 1. FRAGMENTATION SAFE: Appends bytes until a newline is found.
/// 2. UTF-8 SAFE: Uses lossy conversion; never panics on invalid bytes.
/// 3. LATENCY SAFE: Implements LIFO (Last-In-First-Out) to prioritize fresh data.
pub struct GrblStatusBuffer {
    buffer: Vec<u8>,
    capacity: usize,
}

impl GrblStatusBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
            capacity: 1024,
        }
    }

    /// Pushes raw bytes from the hardware into the buffer.
    /// Returns the LATEST valid Z-position if available.
    pub fn push(&mut self, bytes: &[u8]) -> Option<f64> {
        // [SAFETY] We accept bytes directly.

        // 1. Flood Protection (DoS)
        if self.buffer.len() + bytes.len() > self.capacity {
            warn!("SERIAL BUFFER OVERFLOW: Dropping old data");
            self.buffer.clear(); // Hard reset to recover sync
        }

        self.buffer.extend_from_slice(bytes);

        // 2. Scan for complete lines (Delimiter: '\n')
        // We want the LAST complete line (LIFO) to ensure fresh data.
        let mut last_valid_z = None;
        // Iterate over all complete lines in the buffer
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            // Extract the line (including newline for now)
            let line_bytes = &self.buffer[..pos];

            // 3. UTF-8 Safety (The Anti-Bomb)
            // convert_lossy ensures we never panic on garbage bytes
            let line_str = String::from_utf8_lossy(line_bytes);

            // 4. Parse Logic
            if let Some(z) = self.parse_position(&line_str) {
                last_valid_z = Some(z);
            }

            // Advance buffer
            // Remove the line + the newline character (pos + 1)
            // OPTIMIZATION: In a real circular buffer we wouldn't shift O(N),
            // but for <1KB this is safer/simpler than ring pointers.
            self.buffer.drain(..=pos);
        }

        last_valid_z
    }

    /// Pure function to extract Z from a GRBL string.
    /// Input: "<Idle|MPos:10.0,20.0,30.0|FS:0,0>"
    /// Output: Some(30.0)
    fn parse_position(&self, line: &str) -> Option<f64> {
        // [SANITIZATION] Fix Locale Lottery (10,5 -> 10.5)
        // Standard GRBL format: ...MPos:X,Y,Z|...

        // We look for "MPos:"
        if let Some(idx) = line.find("MPos:") {
            // Let's revert to raw line for splitting
            let raw_remainder = &line[idx + 5..];
            let raw_mpos_str = raw_remainder.split('|').next().unwrap_or("");
            let parts: Vec<&str> = raw_mpos_str.split(',').collect();

            if parts.len() >= 3 {
                // We want Z (Index 2)
                let z_str = parts[2];
                // NOW we sanitize the scalar value for Locale issues
                // e.g. "10,5" -> "10.5" (if that ever happens)
                // But typically GRBL is strictly dot-decimal.
                // We mainly guard against weird float formatting.
                let z_clean = z_str.replace(',', ".");

                match z_clean.parse::<f64>() {
                    Ok(val) => return Some(val),
                    Err(_) => {
                        // Silent fail for parsing error, don't crash
                        return None;
                    }
                }
            }
        }
        None
    }
}

// TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragmented_packet() {
        let mut buffer = GrblStatusBuffer::new();

        // Part 1: No newline
        let z = buffer.push(b"<Idle|MPos:10.0,20.0,");
        assert!(z.is_none());

        // Part 2: Completion
        let z = buffer.push(b"30.0|FS:0>\n");
        assert_eq!(z, Some(30.0));
    }

    #[test]
    fn test_garbage_resilience() {
        let mut buffer = GrblStatusBuffer::new();
        // Invalid UTF-8 bytes (0xFF)
        let bytes = b"<Idle|MPos:0,0,50.0|\xFF\n";
        let z = buffer.push(bytes);
        assert_eq!(z, Some(50.0));
    }

    #[test]
    fn test_lifo_freshness() {
        let mut buffer = GrblStatusBuffer::new();
        // Two packets in one push
        let z = buffer.push(b"<Idle|MPos:0,0,10.0|>\n<Idle|MPos:0,0,20.0|>\n");
        // Should return the LATEST (20.0)
        assert_eq!(z, Some(20.0));
    }
}
