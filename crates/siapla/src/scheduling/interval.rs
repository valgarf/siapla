use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, Sub},
};

/// Marker trait for a value that can be used in an interval
pub trait IntervalValue:
    PartialOrd + Ord + PartialEq + Eq + Copy + Clone + std::fmt::Debug
{
}
impl<T: PartialOrd + Ord + PartialEq + Eq + Copy + Clone + std::fmt::Debug> IntervalValue for T {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound<T: IntervalValue> {
    Open(T),
    Closed(T),
    Unbounded(),
}

impl<T: IntervalValue> Bound<T> {
    pub fn value(&self) -> Option<T> {
        match self {
            Bound::Open(v) => Some(*v),
            Bound::Closed(v) => Some(*v),
            Bound::Unbounded() => None,
        }
    }

    pub fn is_unbounded(&self) -> bool {
        *self == Bound::Unbounded()
    }

    pub fn closed(&self) -> Self {
        match self {
            Bound::Open(v) => Bound::Closed(*v),
            v => *v,
        }
    }

    pub fn is_closed(&self) -> bool {
        match self {
            Bound::Closed(_) => true,
            _ => false,
        }
    }

    pub fn switch(&self) -> Self {
        match self {
            Bound::Open(v) => Bound::Closed(*v),
            Bound::Closed(v) => Bound::Open(*v),
            v => *v,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StartBound<T: IntervalValue>(pub Bound<T>);
#[derive(Clone, Copy, Debug)]
pub struct EndBound<T: IntervalValue>(pub Bound<T>);

impl<T: IntervalValue> StartBound<T> {
    pub fn touching_end(&self) -> EndBound<T> {
        match self.0 {
            Bound::Unbounded() => panic!("Cannot find touching end for unbounded start"),
            v => EndBound(v.switch()),
        }
    }
    pub fn value(&self) -> Option<T> {
        self.0.value()
    }
    pub fn closed(&self) -> Self {
        Self(self.0.closed())
    }
    pub fn touches(&self, other: &EndBound<T>) -> bool {
        other.touches(self)
    }
}

impl<T: IntervalValue> EndBound<T> {
    pub fn touching_start(&self) -> StartBound<T> {
        match self.0 {
            Bound::Unbounded() => panic!("Cannot find touching start for unbounded end"),
            v => StartBound(v.switch()),
        }
    }
    pub fn value(&self) -> Option<T> {
        self.0.value()
    }
    pub fn closed(&self) -> Self {
        Self(self.0.closed())
    }
    pub fn touches(&self, other: &StartBound<T>) -> bool {
        if self.0.is_unbounded() || other.0.is_unbounded() {
            false
        } else {
            self == &other.touching_end()
        }
    }
}

macro_rules! ord_standard_impls {
    ($structname: ident) => {
        impl<T: IntervalValue> PartialOrd for $structname<T> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<T: IntervalValue> PartialEq for $structname<T> {
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other) == Ordering::Equal
            }
        }

        impl<T: IntervalValue> Eq for $structname<T> {}
    };
}

impl<T: IntervalValue> Ord for StartBound<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Bound::Open(lhs), Bound::Open(rhs)) => lhs.cmp(&rhs),
            (Bound::Open(lhs), Bound::Closed(rhs)) => {
                if lhs == rhs {
                    Ordering::Greater
                } else {
                    lhs.cmp(&rhs)
                }
            }
            (Bound::Open(_), Bound::Unbounded()) => Ordering::Less,
            (Bound::Closed(lhs), Bound::Open(rhs)) => {
                if lhs == rhs {
                    Ordering::Less
                } else {
                    lhs.cmp(&rhs)
                }
            }
            (Bound::Closed(lhs), Bound::Closed(rhs)) => lhs.cmp(&rhs),
            (Bound::Closed(_), Bound::Unbounded()) => Ordering::Less,
            (Bound::Unbounded(), Bound::Open(_)) => Ordering::Greater,
            (Bound::Unbounded(), Bound::Closed(_)) => Ordering::Greater,
            (Bound::Unbounded(), Bound::Unbounded()) => Ordering::Equal,
        }
    }
}
ord_standard_impls!(StartBound);

impl<T: IntervalValue> Ord for EndBound<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Bound::Open(start), Bound::Open(end)) => start.cmp(&end),
            (Bound::Open(start), Bound::Closed(end)) => {
                if start == end {
                    Ordering::Less
                } else {
                    start.cmp(&end)
                }
            }
            (Bound::Open(_), Bound::Unbounded()) => Ordering::Less,
            (Bound::Closed(start), Bound::Open(end)) => {
                if start == end {
                    Ordering::Greater
                } else {
                    start.cmp(&end)
                }
            }
            (Bound::Closed(start), Bound::Closed(end)) => start.cmp(&end),
            (Bound::Closed(_), Bound::Unbounded()) => Ordering::Less,
            (Bound::Unbounded(), Bound::Open(_)) => Ordering::Greater,
            (Bound::Unbounded(), Bound::Closed(_)) => Ordering::Greater,
            (Bound::Unbounded(), Bound::Unbounded()) => Ordering::Equal,
        }
    }
}
ord_standard_impls!(EndBound);

impl<T: IntervalValue> PartialOrd<&T> for StartBound<T> {
    fn partial_cmp(&self, other: &&T) -> Option<Ordering> {
        match &self.0 {
            Bound::Open(lhs) => {
                if &lhs == other {
                    Some(Ordering::Greater)
                } else {
                    lhs.partial_cmp(other)
                }
            }
            Bound::Closed(start) => start.partial_cmp(other),
            Bound::Unbounded() => Some(Ordering::Less),
        }
    }
}

impl<T: IntervalValue> PartialEq<&T> for StartBound<T> {
    fn eq(&self, other: &&T) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl<T: IntervalValue> PartialOrd<T> for StartBound<T> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.partial_cmp(&other)
    }
}

impl<T: IntervalValue> PartialEq<T> for StartBound<T> {
    fn eq(&self, other: &T) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl<T: IntervalValue> PartialOrd<&T> for EndBound<T> {
    fn partial_cmp(&self, other: &&T) -> Option<Ordering> {
        match &self.0 {
            Bound::Open(lhs) => {
                if &lhs == other {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(other)
                }
            }
            Bound::Closed(start) => start.partial_cmp(other),
            Bound::Unbounded() => Some(Ordering::Less),
        }
    }
}

impl<T: IntervalValue> PartialEq<&T> for EndBound<T> {
    fn eq(&self, other: &&T) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl<T: IntervalValue> PartialOrd<T> for EndBound<T> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.partial_cmp(&other)
    }
}
impl<T: IntervalValue> PartialEq<T> for EndBound<T> {
    fn eq(&self, other: &T) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl<T: IntervalValue> PartialOrd<StartBound<T>> for EndBound<T> {
    fn partial_cmp(&self, other: &StartBound<T>) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (Bound::Open(lhs), Bound::Open(rhs)) => {
                if lhs == rhs {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&rhs)
                }
            }
            (Bound::Open(lhs), Bound::Closed(rhs)) => {
                if lhs == rhs {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&rhs)
                }
            }
            (Bound::Open(_), Bound::Unbounded()) => Some(Ordering::Greater),
            (Bound::Closed(lhs), Bound::Open(rhs)) => {
                if lhs == rhs {
                    Some(Ordering::Less)
                } else {
                    lhs.partial_cmp(&rhs)
                }
            }
            (Bound::Closed(lhs), Bound::Closed(rhs)) => lhs.partial_cmp(&rhs),
            (Bound::Closed(_), Bound::Unbounded()) => Some(Ordering::Greater),
            (Bound::Unbounded(), Bound::Open(_)) => Some(Ordering::Greater),
            (Bound::Unbounded(), Bound::Closed(_)) => Some(Ordering::Greater),
            (Bound::Unbounded(), Bound::Unbounded()) => Some(Ordering::Greater),
        }
    }
}

impl<T: IntervalValue> PartialEq<StartBound<T>> for EndBound<T> {
    fn eq(&self, other: &StartBound<T>) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl<T: IntervalValue> PartialOrd<EndBound<T>> for StartBound<T> {
    fn partial_cmp(&self, other: &EndBound<T>) -> Option<Ordering> {
        match other.partial_cmp(self) {
            Some(res) => Some(match res {
                Ordering::Less => Ordering::Greater,
                Ordering::Equal => Ordering::Equal,
                Ordering::Greater => Ordering::Less,
            }),
            None => None,
        }
    }
}

impl<T: IntervalValue> PartialEq<EndBound<T>> for StartBound<T> {
    fn eq(&self, other: &EndBound<T>) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interval<T: IntervalValue> {
    start: StartBound<T>,
    end: EndBound<T>,
}

impl<T: IntervalValue> Interval<T> {
    pub fn new(start: Bound<T>, end: Bound<T>) -> Self {
        let start = StartBound(start);
        let end = EndBound(end);
        assert!(start < end, "Interval start must be <= end");
        Self { start, end }
    }

    pub fn start(&self) -> Bound<T> {
        self.start.0
    }

    pub fn end(&self) -> Bound<T> {
        self.end.0
    }

    pub fn new_closed(start: T, end: T) -> Self {
        Self::new(Bound::Closed(start), Bound::Closed(end))
    }

    pub fn new_open(start: T, end: T) -> Self {
        Self::new(Bound::Open(start), Bound::Open(end))
    }

    /// left open, right closed
    pub fn new_lorc(start: T, end: T) -> Self {
        Self::new(Bound::Open(start), Bound::Closed(end))
    }

    /// left closed, right open
    pub fn new_lcro(start: T, end: T) -> Self {
        Self::new(Bound::Closed(start), Bound::Open(end))
    }

    pub fn closed(&self) -> Self {
        Self { start: self.start.closed(), end: self.end.closed() }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.start <= value && self.end >= value
    }

    pub fn is_disjoint(&self, other: &Interval<T>) -> bool {
        self.end < other.start || other.end < self.start
    }

    pub fn is_separate(&self, other: &Interval<T>) -> bool {
        (self.end < other.start && !self.end.touches(&other.start))
            || (other.end < self.start && !other.end.touches(&self.start))
    }

    pub fn intersection(&self, other: &Interval<T>) -> Option<Interval<T>> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start <= end { Some(Interval { start, end }) } else { None }
    }

    pub fn union(&self, other: &Interval<T>) -> Option<Interval<T>> {
        if self.is_separate(other) {
            None
        } else {
            Some(Interval { start: self.start.min(other.start), end: self.end.max(other.end) })
        }
    }

    pub fn difference(&self, other: &Interval<T>) -> Vec<Interval<T>> {
        if self.is_disjoint(other) {
            return vec![self.clone()];
        }
        let mut result = Vec::new();
        if self.start < other.start {
            // other.start cannot be unbounded
            result.push(Interval {
                start: self.start,
                end: self.end.min(other.start.touching_end()),
            });
        }
        if self.end > other.end {
            // other.end cannot be unbounded
            result.push(Interval {
                start: self.start.max(other.end.touching_start()),
                end: self.end,
            });
        }
        result
    }
}

impl<T: IntervalValue + Display> Display for Interval<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.start.0 {
            Bound::Open(v) => write!(f, "({}", v)?,
            Bound::Closed(v) => write!(f, "[{}", v)?,
            Bound::Unbounded() => write!(f, "(oo",)?,
        };
        "-".fmt(f)?;
        match self.end.0 {
            Bound::Open(v) => write!(f, "{})", v)?,
            Bound::Closed(v) => write!(f, "{}]", v)?,
            Bound::Unbounded() => write!(f, "oo)",)?,
        };
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Intervals<T: IntervalValue> {
    intervals: Vec<Interval<T>>,
}

impl<T: IntervalValue> Intervals<T> {
    pub fn new() -> Self {
        Intervals { intervals: Vec::new() }
    }

    /// Efficient binary search since intervals are sorted and non-overlapping.
    pub fn contains(&self, value: &T) -> bool {
        self.find_index(value).is_ok()
    }

    /// Returns the index of the interval containing value, or None if not found.
    fn find_index(&self, value: &T) -> Result<usize, usize> {
        self.intervals.binary_search_by(|iv| {
            if iv.start > value {
                std::cmp::Ordering::Greater
            } else if iv.end < value {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
    }

    pub fn find(&self, value: &T) -> Option<&Interval<T>> {
        self.find_index(value).ok().map(|i| &self.intervals[i])
    }

    /// Efficient binary search since intervals are sorted and non-overlapping.
    pub fn touches(&self, value: &T) -> bool {
        self.find_index_touching(value).is_ok()
    }

    /// Returns the index of the interval containing or touching value, or None if not found.
    fn find_index_touching(&self, value: &T) -> Result<usize, usize> {
        self.intervals.binary_search_by(|iv| {
            let iv_closed = iv.closed();
            if iv_closed.start > value {
                std::cmp::Ordering::Greater
            } else if iv_closed.end < value {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
    }

    pub fn find_touching(&self, value: &T) -> Option<&Interval<T>> {
        self.find_index_touching(value).ok().map(|i| &self.intervals[i])
    }

    /// Check if the other intervals do not have any timepoints in common.
    pub fn is_disjoint(&self, other: &Intervals<T>) -> bool {
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() && j < other.intervals.len() {
            let a = &self.intervals[i];
            let b = &other.intervals[j];
            if a.is_disjoint(b) {
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

    /// Check if the other intervals do not have any timepoints in common and are not touching
    /// any of these intervals.
    pub fn is_separate(&self, other: &Intervals<T>) -> bool {
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

    pub fn union(&self, other: &Intervals<T>) -> Intervals<T> {
        let mut merged: Vec<Interval<T>> = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() || j < other.intervals.len() {
            let next = match (self.intervals.get(i), other.intervals.get(j)) {
                (Some(a), Some(b)) => {
                    if let Some(c) = a.union(b) {
                        i += 1;
                        j += 1;
                        c
                    } else if a.start < b.start {
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

    pub fn hull(&self) -> Option<Interval<T>> {
        if let (
            Some(Interval { start: first_start, end: _ }),
            Some(Interval { start: _, end: last_end }),
        ) = (self.intervals.first(), self.intervals.last())
        {
            Some(Interval { start: *first_start, end: *last_end })
        } else {
            None
        }
    }

    pub fn intersection(&self, other: &Intervals<T>) -> Intervals<T> {
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

    pub fn difference(&self, other: &Intervals<T>) -> Intervals<T> {
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
            if j + 1 < k {
                j = k - 1;
            }
        }
        Intervals { intervals: result }
    }

    /// Insert a new interval, merging with overlapping or adjacent intervals to keep the vector ordered and disjoint.
    pub fn insert(&mut self, mut new_iv: Interval<T>) {
        // Find the first interval that could overlap or be adjacent
        let i = match new_iv.start.0.value() {
            Some(v) => self.find_index_touching(&v).unwrap_or_else(|idx| idx),
            None => 0, // unbounded
        };

        // Merge with all overlapping or adjacent intervals
        let mut j = i;
        while j < self.intervals.len()
            && (self.intervals[j].start <= new_iv.end
                || self.intervals[j].start.touches(&new_iv.end))
        {
            new_iv.start = new_iv.start.min(self.intervals[j].start);
            new_iv.end = new_iv.end.max(self.intervals[j].end);
            j += 1;
        }
        // Remove merged intervals
        self.intervals.splice(i..j, [new_iv]);
    }

    /// Remove the given interval from all intervals, keeping the vector ordered and separate.
    pub fn remove(&mut self, interval: Interval<T>) {
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

    /// Create two new intervals with the given interval removed.
    pub fn split_remove(&self, interval: Interval<T>) -> (Intervals<T>, Intervals<T>) {
        // TODO: add tests for this
        let mut lhs = vec![];
        let mut rhs = vec![];
        for iv in &self.intervals {
            if iv.end < interval.start {
                lhs.push(iv.clone())
            } else if iv.start > interval.end {
                rhs.push(iv.clone());
            } else {
                for iv in iv.difference(&interval) {
                    if iv.start < interval.start {
                        lhs.push(iv.clone())
                    } else if iv.end > interval.end {
                        rhs.push(iv.clone());
                    }
                }
            }
        }
        (Intervals { intervals: lhs }, Intervals { intervals: rhs })
    }
}

impl<T: IntervalValue> FromIterator<Interval<T>> for Intervals<T> {
    fn from_iter<I: IntoIterator<Item = Interval<T>>>(iter: I) -> Self {
        // TODO: this can be optimized
        let mut result = Self::new();
        for iv in iter {
            result.insert(iv);
        }
        result
    }
}

impl<O, T: IntervalValue + Sub<Output = O>> Interval<T> {
    /// Length of the interval (i.e. `end-start`), None if start or end are unbounded.
    pub fn length(&self) -> Option<O> {
        if let (Some(start), Some(end)) = (self.start.0.value(), self.end.0.value()) {
            Some(end - start)
        } else {
            None
        }
    }
}

impl<O: Add<Output = O> + Default, T: IntervalValue + Sub<Output = O>> Intervals<T> {
    /// Sum of the lengths of the individual intervals. None if any of them is unbounded.
    pub fn length(&self) -> Option<O> {
        let result =
            self.intervals.iter().map(|iv| iv.length()).try_fold(O::default(), |acc, el| {
                if let Some(el) = el { Ok(acc + el) } else { Err(()) }
            });
        result.ok()
    }
}

impl<T: IntervalValue> IntoIterator for Intervals<T> {
    type Item = Interval<T>;

    type IntoIter = <Vec<Interval<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.intervals.into_iter()
    }
}

impl<'a, T: IntervalValue> IntoIterator for &'a Intervals<T> {
    type Item = &'a Interval<T>;

    type IntoIter = <&'a Vec<Interval<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.intervals).into_iter()
    }
}

impl<'a, T: IntervalValue> IntoIterator for &'a mut Intervals<T> {
    type Item = &'a mut Interval<T>;

    type IntoIter = <&'a mut Vec<Interval<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.intervals).into_iter()
    }
}

impl<T: IntervalValue> From<Interval<T>> for Intervals<T> {
    fn from(value: Interval<T>) -> Self {
        Intervals { intervals: vec![value] }
    }
}

impl<T: IntervalValue + Display> Display for Intervals<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "{".fmt(f)?;
        let mut is_first = true;
        for iv in &self.intervals {
            if !is_first {
                "|".fmt(f)?;
            }
            is_first = false;
            iv.fmt(f)?;
        }
        "}".fmt(f)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::NaiveDate;
    use chrono::NaiveDateTime;

    // TODO: tests for intervals other than lcro (currently not used in the code)

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
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        assert!(a.contains(&ndt("2023-01-01", "00:00:00")));
        assert!(a.contains(&ndt("2023-01-01", "11:59:59")));
        assert!(!a.contains(&ndt("2023-01-01", "12:00:00")));
        assert!(!a.contains(&ndt("2022-12-31", "23:59:59")));
    }

    #[test]
    fn test_interval_is_disjoint() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "12:00:00"), ndt("2023-01-01", "13:00:00"));
        let c = Interval::new_lcro(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let d = Interval::new_lcro(ndt("2023-01-01", "13:00:00"), ndt("2023-01-01", "15:00:00"));
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
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let u = a.union(&b).unwrap();
        assert_eq!(u.start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(u.end.value(), Some(ndt("2023-01-01", "13:00:00")));
    }

    #[test]
    fn test_union_touching_intervals() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00"));
        let u = a.union(&b).unwrap();
        assert_eq!(u.start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(u.end.value(), Some(ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_union_separate_intervals() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "07:00:00"), ndt("2023-01-01", "10:00:00"));
        assert!(a.union(&b).is_none());
    }

    #[test]
    fn test_interval_intersection() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "11:00:00"), ndt("2023-01-01", "13:00:00"));
        let i = a.intersection(&b).unwrap();
        assert_eq!(i.start.value(), Some(ndt("2023-01-01", "11:00:00")));
        assert_eq!(i.end.value(), Some(ndt("2023-01-01", "12:00:00")));
    }

    #[test]
    fn test_intersection_touching_intervals() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "10:00:00"));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_interval_difference() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "08:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(diff[0].end.value(), Some(ndt("2023-01-01", "06:00:00")));
        assert_eq!(diff[1].start.value(), Some(ndt("2023-01-01", "08:00:00")));
        assert_eq!(diff[1].end.value(), Some(ndt("2023-01-01", "12:00:00")));
    }

    #[test]
    fn test_interval_difference_stretch_right() {
        let a = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "15:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(diff[0].end.value(), Some(ndt("2023-01-01", "06:00:00")));
    }

    #[test]
    fn test_interval_difference_stretch_left() {
        let a = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "08:00:00"));
        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].start.value(), Some(ndt("2023-01-01", "08:00:00")));
        assert_eq!(diff[0].end.value(), Some(ndt("2023-01-01", "12:00:00")));
    }

    #[test]
    fn test_interval_difference_touching() {
        let a = Interval::new_lcro(ndt("2023-01-01", "06:00:00"), ndt("2023-01-01", "12:00:00"));
        let b = Interval::new_lcro(ndt("2023-01-01", "12:00:00"), ndt("2023-01-01", "15:00:00"));
        let c = Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00"));
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
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "06:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "08:00:00"),
            ndt("2023-01-01", "10:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "05:00:00"),
            ndt("2023-01-01", "09:00:00"),
        ));
        // Should merge to [00:00:00, 10:00:00)
        assert_eq!(intervals.intervals.len(), 1);
        assert!(intervals.contains(&ndt("2023-01-01", "09:59:59")));
        assert!(!intervals.contains(&ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_find_index() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "06:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "08:00:00"),
            ndt("2023-01-01", "10:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "10:00:00"),
            ndt("2023-01-01", "11:00:00"),
        ));
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
        a.insert(Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new_lcro(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "10:00:00")));
        let u = a.union(&b);
        assert_eq!(u.intervals.len(), 1);
        assert_eq!(u.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(u.intervals[0].end.value(), Some(ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_intersection() {
        let mut a = Intervals::new();
        a.insert(Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "06:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new_lcro(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "10:00:00")));
        let i = a.intersection(&b);
        assert_eq!(i.intervals.len(), 1);
        assert_eq!(i.intervals[0].start.value(), Some(ndt("2023-01-01", "05:00:00")));
        assert_eq!(i.intervals[0].end.value(), Some(ndt("2023-01-01", "06:00:00")));
    }

    #[test]
    fn test_intervals_difference() {
        let mut a = Intervals::new();
        a.insert(Interval::new_lcro(ndt("2023-01-01", "00:00:00"), ndt("2023-01-01", "10:00:00")));
        let mut b = Intervals::new();
        b.insert(Interval::new_lcro(ndt("2023-01-01", "05:00:00"), ndt("2023-01-01", "07:00:00")));
        let d = a.difference(&b);
        assert_eq!(d.intervals.len(), 2);
        assert_eq!(d.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(d.intervals[0].end.value(), Some(ndt("2023-01-01", "05:00:00")));
        assert_eq!(d.intervals[1].start.value(), Some(ndt("2023-01-01", "07:00:00")));
        assert_eq!(d.intervals[1].end.value(), Some(ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_insert_merge_two_disjoint_int() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(1, 3));
        intervals.insert(Interval::new_lcro(5, 9));
        assert_eq!(intervals.intervals.len(), 2);
        // Insert a third interval that touches both
        intervals.insert(Interval::new_lcro(3, 5));
        // Should merge to [00:00:00, 12:00:00)
        assert_eq!(intervals.intervals.len(), 1);
        assert_eq!(intervals.intervals[0].start.value(), Some(1));
        assert_eq!(intervals.intervals[0].end.value(), Some(9));
    }

    #[test]
    fn test_intervals_insert_merge_two_disjoint() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "06:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "10:00:00"),
            ndt("2023-01-01", "12:00:00"),
        ));
        assert_eq!(intervals.intervals.len(), 2);
        // Insert a third interval that touches both
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "06:00:00"),
            ndt("2023-01-01", "10:00:00"),
        ));
        // Should merge to [00:00:00, 12:00:00)
        assert_eq!(intervals.intervals.len(), 1);
        assert_eq!(intervals.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(intervals.intervals[0].end.value(), Some(ndt("2023-01-01", "12:00:00")));
    }

    #[test]
    fn test_intervals_remove_subinterval() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "10:00:00"),
        ));
        intervals.remove(Interval::new_lcro(
            ndt("2023-01-01", "03:00:00"),
            ndt("2023-01-01", "07:00:00"),
        ));
        assert_eq!(intervals.intervals.len(), 2);
        assert_eq!(intervals.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(intervals.intervals[0].end.value(), Some(ndt("2023-01-01", "03:00:00")));
        assert_eq!(intervals.intervals[1].start.value(), Some(ndt("2023-01-01", "07:00:00")));
        assert_eq!(intervals.intervals[1].end.value(), Some(ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_remove_overlap_multiple() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "04:00:00"),
        ));
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "06:00:00"),
            ndt("2023-01-01", "10:00:00"),
        ));
        intervals.remove(Interval::new_lcro(
            ndt("2023-01-01", "03:00:00"),
            ndt("2023-01-01", "07:00:00"),
        ));
        assert_eq!(intervals.intervals.len(), 2);
        assert_eq!(intervals.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(intervals.intervals[0].end.value(), Some(ndt("2023-01-01", "03:00:00")));
        assert_eq!(intervals.intervals[1].start.value(), Some(ndt("2023-01-01", "07:00:00")));
        assert_eq!(intervals.intervals[1].end.value(), Some(ndt("2023-01-01", "10:00:00")));
    }

    #[test]
    fn test_intervals_remove_disjoint() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "04:00:00"),
        ));
        intervals.remove(Interval::new_lcro(
            ndt("2023-01-01", "05:00:00"),
            ndt("2023-01-01", "06:00:00"),
        ));
        assert_eq!(intervals.intervals.len(), 1);
        assert_eq!(intervals.intervals[0].start.value(), Some(ndt("2023-01-01", "00:00:00")));
        assert_eq!(intervals.intervals[0].end.value(), Some(ndt("2023-01-01", "04:00:00")));
    }

    #[test]
    fn test_intervals_remove_exact() {
        let mut intervals = Intervals::new();
        intervals.insert(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "04:00:00"),
        ));
        intervals.remove(Interval::new_lcro(
            ndt("2023-01-01", "00:00:00"),
            ndt("2023-01-01", "04:00:00"),
        ));
        assert!(intervals.intervals.is_empty());
    }
}
