use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
pub struct Fraction(pub u32, pub u32);

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&(self.0 * other.1), &(other.0 * self.1))
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Fraction {}

impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
