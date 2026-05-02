use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct SrcLoc {
    pos: usize,
}

impl SrcLoc {
    pub fn new(pos: usize) -> Self {
        return SrcLoc {
            pos,
        }
    }
}

impl fmt::Display for SrcLoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.pos)
    }
}
