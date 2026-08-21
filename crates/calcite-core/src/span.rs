#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "span start must not exceed its end");
        Self { start, end }
    }

    pub const fn empty(at: usize) -> Self {
        Self::new(at, at)
    }

    pub fn join(self, other: Self) -> Self {
        Self::new(self.start.min(other.start), self.end.max(other.end))
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub(crate) const fn shifted(self, offset: usize) -> Self {
        Self::new(self.start + offset, self.end + offset)
    }
}
