use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// A HashSet that stores Weak references to Arc<T>.
/// On iteration, dead weak pointers are removed.
#[derive(Debug, Clone)]
pub struct WeakHashSet<T: ?Sized> {
    set: HashSet<WeakKey<T>>,
}

/// Wrapper for Weak<T> to implement Hash and Eq based on pointer address.
#[derive(Clone, Debug)]
struct WeakKey<T: ?Sized> {
    weak: Weak<T>,
}

impl<T: ?Sized> PartialEq for WeakKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.weak.ptr_eq(&other.weak)
    }
}

impl<T: ?Sized> Eq for WeakKey<T> {}

impl<T: ?Sized> Hash for WeakKey<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the address of the allocation
        if let Some(strong) = self.weak.upgrade() {
            let ptr = Arc::as_ptr(&strong) as *const ();
            (ptr as usize).hash(state);
        } else {
            // Hash a unique value for dead weak pointers
            0usize.hash(state);
        }
    }
}

impl<T: ?Sized> WeakHashSet<T> {
    pub fn new() -> Self {
        Self {
            set: HashSet::new(),
        }
    }

    pub fn insert(&mut self, value: &Arc<T>) -> bool {
        self.set.insert(WeakKey {
            weak: Arc::downgrade(value),
        })
    }

    pub fn remove(&mut self, value: &Arc<T>) -> bool {
        self.set.remove(&WeakKey {
            weak: Arc::downgrade(value),
        })
    }

    pub fn contains(&self, value: &Arc<T>) -> bool {
        self.set.contains(&WeakKey {
            weak: Arc::downgrade(value),
        })
    }

    /// Returns an iterator over all live Arc<T> in the set, cleaning up dead entries.
    pub fn iter(&mut self) -> impl Iterator<Item = Arc<T>> + '_ {
        self.cleanup();
        self.set.iter().filter_map(|wk| wk.weak.upgrade())
    }

    pub fn cleanup(&mut self) {
        self.set.retain(|wk| wk.weak.strong_count() > 0);
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}
