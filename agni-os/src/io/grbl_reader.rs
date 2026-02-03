use regex::Regex;

const MAX_BUFFER_SIZE: usize = 4096;

pub struct GrblStatusBuffer {
    buf: String,
    re: Regex,
}

impl GrblStatusBuffer {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            // Capture 3rd float (Z-Axis) -> MPos:X,Y,Z
            re: Regex::new(r"MPos:[-\d.]+,[-\d.]+,([-\d.]+)").unwrap(),
        }
    }

    pub fn push(&mut self, data: &str) -> Option<f64> {
        // D-05: Unbounded Buffer Growth Fix
        if self.buf.len() + data.len() > MAX_BUFFER_SIZE {
            // Fix Defect in Reader:
            // Clearing buffer causes data loss. We should retain the end.
            // Keep last 1024 bytes (enough for a few frames)
            let keep = 1024.min(self.buf.len());
            let start = self.buf.len() - keep;
            let saved = self.buf[start..].to_string();
            self.buf = saved;
            // self.buf.push_str(data) happens next
        }

        self.buf.push_str(data);

        // Scan for line ending
        if let Some(end) = self.buf.find('>') {
            let frame = self.buf[..=end].to_string();
            // Advance buffer
            self.buf = self.buf[end + 1..].to_string();

            // Extract Z
            if let Some(cap) = self.re.captures(&frame) {
                // Return Z in mm (GRBL default)
                return cap[1].parse::<f64>().ok();
            }
        }
        None
    }
}
