use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interval {
    start: NaiveDateTime,
    end: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Intervals {
    intervals: Vec<Interval>,
}

impl Interval {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        assert!(start < end, "Interval start must be < end");
        Interval { start, end }
    }

    pub fn contains(&self, dt: &NaiveDateTime) -> bool {
        &self.start <= dt && dt < &self.end
    }

    pub fn is_disjoint(&self, other: &Interval) -> bool {
        self.end <= other.start || other.end <= self.start
    }

    pub fn is_separate(&self, other: &Interval) -> bool {
        self.end < other.start || other.end < self.start
    }

    pub fn intersection(&self, other: &Interval) -> Option<Interval> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end { Some(Interval { start, end }) } else { None }
    }

    pub fn union(&self, other: &Interval) -> Option<Interval> {
        if self.is_separate(other) {
            None
        } else {
            Some(Interval { start: self.start.min(other.start), end: self.end.max(other.end) })
        }
    }

    pub fn difference(&self, other: &Interval) -> Vec<Interval> {
        if self.is_disjoint(other) {
            return vec![self.clone()];
        }
        let mut result = Vec::new();
        if self.start < other.start {
            result.push(Interval { start: self.start, end: self.end.min(other.start) });
        }
        if self.end > other.end {
            result.push(Interval { start: self.start.max(other.end), end: self.end });
        }
        result
    }
}

impl Intervals {
    pub fn new() -> Self {
        Intervals { intervals: Vec::new() }
    }

    /// Efficient binary search since intervals are sorted and non-overlapping.
    pub fn contains(&self, dt: &NaiveDateTime) -> bool {
        self.find_index(dt).is_ok()
    }

    /// Returns the index of the interval containing dt, or None if not found.
    fn find_index(&self, dt: &NaiveDateTime) -> Result<usize, usize> {
        self.intervals.binary_search_by(|iv| {
            if dt < &iv.start {
                std::cmp::Ordering::Greater
            } else if dt >= &iv.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
    }

    pub fn find(&self, dt: &NaiveDateTime) -> Option<&Interval> {
        self.find_index(dt).ok().map(|i| &self.intervals[i])
    }

    /// Efficient binary search since intervals are sorted and non-overlapping.
    pub fn touches(&self, dt: &NaiveDateTime) -> bool {
        self.find_index_touching(dt).is_ok()
    }

    /// Returns the index of the interval containing dt, or None if not found.
    fn find_index_touching(&self, dt: &NaiveDateTime) -> Result<usize, usize> {
        self.intervals.binary_search_by(|iv| {
            if dt < &iv.start {
                std::cmp::Ordering::Greater
            } else if dt > &iv.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
    }

    pub fn find_touching(&self, dt: &NaiveDateTime) -> Option<&Interval> {
        self.find_index_touching(dt).ok().map(|i| &self.intervals[i])
    }

    pub fn is_disjoint(&self, other: &Intervals) -> bool {
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() && j < other.intervals.len() {
            let a = &self.intervals[i];
            let b = &other.intervals[j];
            if a.is_disjoint(b) {
                if a.end <= b.start {
                    i += 1;
                } else {
                    j += 1;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn is_separate(&self, other: &Intervals) -> bool {
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() && j < other.intervals.len() {
            let a = &self.intervals[i];
            let b = &other.intervals[j];
            if a.is_separate(b) {
                if a.end < b.start {
                    i += 1;
                } else {
                    j += 1;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn union(&self, other: &Intervals) -> Intervals {
        let mut merged: Vec<Interval> = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() || j < other.intervals.len() {
            let next = match (self.intervals.get(i), other.intervals.get(j)) {
                (Some(a), Some(b)) => {
                    if let Some(c) = a.union(b) {
                        i += 1;
                        j += 1;
                        c
                    } else if a.start <= b.start {
                        i += 1;
                        a.clone()
                    } else {
                        j += 1;
                        b.clone()
                    }
                }
                (Some(a), None) => {
                    i += 1;
                    a.clone()
                }
                (None, Some(b)) => {
                    j += 1;
                    b.clone()
                }
                (None, None) => break,
            };
            if let Some(last) = merged.last_mut() {
                if let Some(u) = last.union(&next) {
                    *last = u;
                    continue;
                }
            }
            merged.push(next);
        }
        Intervals { intervals: merged }
    }

    pub fn intersection(&self, other: &Intervals) -> Intervals {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() && j < other.intervals.len() {
            let a = &self.intervals[i];
            let b = &other.intervals[j];
            if let Some(iv) = a.intersection(b) {
                result.push(iv);
            }
            if a.end < b.end {
                i += 1;
            } else {
                j += 1;
            }
        }
        Intervals { intervals: result }
    }

    pub fn difference(&self, other: &Intervals) -> Intervals {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() {
            let a = self.intervals[i].clone();
            while j < other.intervals.len() && other.intervals[j].end < a.start {
                j += 1;
            }
            let mut k = j;
            let mut diffs = vec![a.clone()];
            while k < other.intervals.len() && other.intervals[k].start <= a.end {
                diffs =
                    diffs.into_iter().flat_map(|iv| iv.difference(&other.intervals[k])).collect();
                k += 1;
            }
            for d in diffs {
                result.push(d);
            }
            i += 1;
        }
        Intervals { intervals: result }
    }

    /// Insert a new interval, merging with overlapping or adjacent intervals to keep the vector ordered and disjoint.
    pub fn insert(&mut self, mut new_iv: Interval) {
        // Find the first interval that could overlap or be adjacent
        let i = self.find_index_touching(&new_iv.start).unwrap_or_else(|idx| idx);

        // Merge with all overlapping or adjacent intervals
        let mut j = i;
        while j < self.intervals.len() && self.intervals[j].start <= new_iv.end {
            new_iv.start = new_iv.start.min(self.intervals[j].start);
            new_iv.end = new_iv.end.max(self.intervals[j].end);
            j += 1;
        }
        // Remove merged intervals
        self.intervals.splice(i..j, [new_iv]);
    }

    /// Remove the given interval from all intervals, keeping the vector ordered and separate.
    pub fn remove(&mut self, interval: Interval) {
        let mut new_intervals = Vec::new();
        for iv in self.intervals.drain(..) {
            if iv.is_disjoint(&interval) {
                new_intervals.push(iv);
            } else {
                new_intervals.extend(iv.difference(&interval));
            }
        }
        self.intervals = new_intervals;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn ndt(date: &str, time: &str) -> NaiveDateTime {
        NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(
                time[0..2].parse().unwrap(),
                time[3..5].parse().unwrap(),
                time[6..8].parse().unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn test_interval_contains() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        assert!(a.contains(&ndt("2023-01-01", "00:00:00")));
        assert!(a.contains(&ndt("2023-01-01", "11:59:59")));
        assert!(!a.contains(&ndt("2023-01-01", "12:00:00")));
        assert!(!a.contains(&ndt("2022-12-31", "23:59:59")));
    }

    #[test]
    fn test_interval_is_disjoint() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "12:00:00"), ndt("2023-01-01", "13:00:00"));
        let c = Interval::new(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let d = Interval::new(ndt("2023-01-01", "13:00:00"), ndt("2023-01-01", "15:00:00"));
        assert!(a.is_disjoint(&b));
        assert!(!a.is_disjoint(&c));
        assert!(a.is_disjoint(&d));
        assert!(b.is_disjoint(&d));
        assert!(!a.is_separate(&b));
        assert!(!a.is_separate(&c));
        assert!(a.is_separate(&d));
        assert!(!b.is_separate(&d));
    }

    #[test]
    fn test_interval_union() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let u = a.union(&b).unwrap();
        assert_eq!(u.start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(u.end, ndt("2023-01-01", "13:00:00"));
    }

    #[test]
    fn test_union_touching_intervals() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00"));
        let u = a.union(&b).unwrap();
        assert_eq!(u.start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(u.end, ndt("2023-01-01", "10:00:00"));
    }

    #[test]
    fn test_union_separate_intervals() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new(ndt("2023-01-01", "07:00:00"), ndt("2023-01-01", "10:00:00"));
        assert!(a.union(&b).is_none());
    }

    #[test]
    fn test_interval_intersection() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let i = a.intersection(&b).unwrap();
        assert_eq!(i.start, ndt("2023-01-01", "11:00:00"));
        assert_eq!(i.end, ndt("2023-01-01", "12:00:00"));
    }

    #[test]
    fn test_intersection_touching_intervals() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00"));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_interval_difference() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "08:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(diff[0].end, ndt("2023-01-01", "06:00:00"));
        assert_eq!(diff[1].start, ndt("2023-01-01", "08:00:00"));
        assert_eq!(diff[1].end, ndt("2023-01-01", "12:00:00"));
    }

    #[test]
    fn test_interval_difference_stretch_right() {
        let a = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "15:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(diff[0].end, ndt("2023-01-01", "06:00:00"));
    }

    #[test]
    fn test_interval_difference_stretch_left() {
        let a = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "08:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].start, ndt("2023-01-01", "08:00:00"));
        assert_eq!(diff[0].end, ndt("2023-01-01", "12:00:00"));
    }

    #[test]
    fn test_interval_difference_touching() {
        let a = Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new(ndt("2023-01-01", "12:00:00"), ndt("2023-01-01", "15:00:00"));
        let c = Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0], a);
        let diff = a.difference(&c);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0], a);
    }

    #[test]
    fn test_intervals_insert_and_contains() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "08:00:00"), ndt("2023-01-01", "10:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "09:00:00")));
        // Should merge to [00:00:00, 10:00:00)
        assert_eq!(intervals.intervals.len(), 1);
        assert!(intervals.contains(&ndt("2023-01-01", "09:59:59")));
        assert!(!intervals.contains(&ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_find_index() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "08:00:00"), ndt("2023-01-01", "10:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "10:00:00"), ndt("2023-01-01", "11:00:00")));
        assert_eq!(intervals.intervals.len(), 2);
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "05:00:00")).ok(), Some(0));
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "08:30:00")).ok(), Some(1));
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "10:00:00")).ok(), Some(1));
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "10:30:00")).ok(), Some(1));
        assert_eq!(intervals.find_index(&ndt("2022-12-23", "23:59:59")).err(), Some(0));
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "07:00:00")).err(), Some(1));
        assert_eq!(intervals.find_index(&ndt("2023-01-01", "12:00:00")).err(), Some(2));
    }

    #[test]
    fn test_intervals_union() {
        let mut a = Intervals::new();
        a.insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "10:00:00")));
        let u = a.union(&b);
        assert_eq!(u.intervals.len(), 1);
        assert_eq!(u.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(u.intervals[0].end, ndt("2023-01-01", "10:00:00"));
    }

    #[test]
    fn test_intervals_intersection() {
        let mut a = Intervals::new();
        a.insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "10:00:00")));
        let i = a.intersection(&b);
        assert_eq!(i.intervals.len(), 1);
        assert_eq!(i.intervals[0].start, ndt("2023-01-01", "05:00:00"));
        assert_eq!(i.intervals[0].end, ndt("2023-01-01", "06:00:00"));
    }

    #[test]
    fn test_intervals_difference() {
        let mut a = Intervals::new();
        a.insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "10:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "07:00:00")));
        let d = a.difference(&b);
        assert_eq!(d.intervals.len(), 2);
        assert_eq!(d.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(d.intervals[0].end, ndt("2023-01-01", "05:00:00"));
        assert_eq!(d.intervals[1].start, ndt("2023-01-01", "07:00:00"));
        assert_eq!(d.intervals[1].end, ndt("2023-01-01", "10:00:00"));
    }

    #[test]
    fn test_intervals_insert_merge_two_disjoint() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "10:00:00"), ndt("2023-01-01", "12:00:00")));
        assert_eq!(intervals.intervals.len(), 2);
        // Insert a third interval that touches both
        intervals
            .insert(Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00")));
        // Should merge to [00:00:00, 12:00:00)
        assert_eq!(intervals.intervals.len(), 1);
        assert_eq!(intervals.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(intervals.intervals[0].end, ndt("2023-01-01", "12:00:00"));
    }

    #[test]
    fn test_intervals_remove_subinterval() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "10:00:00")));
        intervals
            .remove(Interval::new(ndt("2023-01-01", "03:00:00"), ndt("2023-01-01", "07:00:00")));
        assert_eq!(intervals.intervals.len(), 2);
        assert_eq!(intervals.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(intervals.intervals[0].end, ndt("2023-01-01", "03:00:00"));
        assert_eq!(intervals.intervals[1].start, ndt("2023-01-01", "07:00:00"));
        assert_eq!(intervals.intervals[1].end, ndt("2023-01-01", "10:00:00"));
    }

    #[test]
    fn test_intervals_remove_overlap_multiple() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "04:00:00")));
        intervals
            .insert(Interval::new(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00")));
        intervals
            .remove(Interval::new(ndt("2023-01-01", "03:00:00"), ndt("2023-01-01", "07:00:00")));
        assert_eq!(intervals.intervals.len(), 2);
        assert_eq!(intervals.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(intervals.intervals[0].end, ndt("2023-01-01", "03:00:00"));
        assert_eq!(intervals.intervals[1].start, ndt("2023-01-01", "07:00:00"));
        assert_eq!(intervals.intervals[1].end, ndt("2023-01-01", "10:00:00"));
    }

    #[test]
    fn test_intervals_remove_disjoint() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "04:00:00")));
        intervals
            .remove(Interval::new(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "06:00:00")));
        assert_eq!(intervals.intervals.len(), 1);
        assert_eq!(intervals.intervals[0].start, ndt("2023-01-01", "00:00:00"));
        assert_eq!(intervals.intervals[0].end, ndt("2023-01-01", "04:00:00"));
    }

    #[test]
    fn test_intervals_remove_exact() {
        let mut intervals = Intervals::new();
        intervals
            .insert(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "04:00:00")));
        intervals
            .remove(Interval::new(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "04:00:00")));
        assert!(intervals.intervals.is_empty());
    }
}
