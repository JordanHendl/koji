use dashi::utils::*;

pub struct ResourceList<T> {
    pub pool: Pool<T>,
    pub entries: Vec<Handle<T>>,
}

impl<T> Default for ResourceList<T> {
    fn default() -> Self {
        Self {
            pool: Default::default(),
            entries: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<T> ResourceList<T> {
    pub fn new(size: usize) -> Self {
        Self {
            pool: Pool::new(size),
            entries: Vec::with_capacity(size),
        }
    }

    pub fn push(&mut self, v: T) -> Handle<T> {
        let h = self.pool.insert(v).unwrap();
        self.entries.push(h);
        h
    }

    pub fn release(&mut self, h: Handle<T>) {
        if let Some(idx) = self.entries.iter().position(|a| a.slot == h.slot) {
            self.entries.remove(idx);
            self.pool.release(h);
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get_ref(&self, h: Handle<T>) -> &T {
        self.pool.get_ref(h).unwrap()
    }

    pub fn get_ref_mut(&mut self, h: Handle<T>) -> &mut T {
        self.pool.get_mut_ref(h).unwrap()
    }

    #[allow(dead_code)]
    pub fn for_each_occupied<F>(&self, func: F)
    where
        F: Fn(&T),
    {
        for item in &self.entries {
            let r = self.pool.get_ref(item.clone()).unwrap();
            func(r);
        }
    }

    pub fn for_each_handle<F>(&self, mut func: F)
    where
        F: FnMut(Handle<T>),
    {
        for h in &self.entries {
            func(*h);
        }
    }

    #[allow(dead_code)]
    pub fn for_each_occupied_mut<F>(&mut self, mut func: F)
    where
        F: FnMut(&T),
    {
        for item in &self.entries {
            let r = self.pool.get_mut_ref(item.clone()).unwrap();
            func(r);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries
            .iter()
            .map(move |h| self.pool.get_ref(*h).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_len() {
        let mut list = ResourceList::default();
        let h1 = list.push(1u32);
        let h2 = list.push(2u32);
        let h3 = list.push(3u32);

        assert_eq!(list.len(), 3);
        assert_eq!(*list.get_ref(h1), 1);
        assert_eq!(*list.get_ref(h2), 2);
        assert_eq!(*list.get_ref(h3), 3);
    }

    #[test]
    fn release_and_pool_state() {
        let mut list = ResourceList::default();
        let h1 = list.push(10u32);
        let h2 = list.push(20u32);

        assert_eq!(list.len(), 2);

        list.release(h1);
        assert_eq!(list.len(), 1);

        // releasing the same handle again should have no effect
        list.release(h1);
        assert_eq!(list.len(), 1);

        let h3 = list.push(30u32);
        assert_eq!(list.len(), 2);

        assert_eq!(*list.get_ref(h2), 20);
        assert_eq!(*list.get_ref(h3), 30);

        // released handle now refers to the newly inserted value
        assert_eq!(*list.get_ref(h1), 30);
    }

    #[test]
    fn get_ref_mut_and_panic_after_release() {
        let mut list = ResourceList::default();
        let h = list.push(5u32);

        // mutate through mutable reference
        *list.get_ref_mut(h) = 6;
        assert_eq!(*list.get_ref(h), 6);

        list.release(h);

        // invalid handle should panic
        assert!(std::panic::catch_unwind(|| list.get_ref(Handle::default())).is_err());
        use std::panic::AssertUnwindSafe;
        assert!(std::panic::catch_unwind(AssertUnwindSafe(|| {
            list.get_ref_mut(Handle::default());
        })).is_err());
    }

    #[test]
    fn iteration_methods() {
        let mut list = ResourceList::default();
        let mut handles = Vec::new();
        for i in 0u32..5 {
            handles.push(list.push(i));
        }

        let collected: Vec<_> = list.iter().copied().collect();
        assert_eq!(collected, (0u32..5).collect::<Vec<_>>());

        let mut handle_list = Vec::new();
        list.for_each_handle(|h| handle_list.push(h));
        assert_eq!(handle_list, handles);

        use std::cell::Cell;
        let index = Cell::new(0u32);
        list.for_each_occupied(|v| {
            let i = index.get();
            assert_eq!(*v, i);
            index.set(i + 1);
        });

        // remove one handle and ensure iterators reflect the change
        list.release(handles[2]);
        let after_release: Vec<_> = list.iter().copied().collect();
        assert_eq!(after_release.len(), 4);
        assert!(after_release.iter().all(|&v| v != 2));
    }

    #[test]
    fn release_nonexistent_handle() {
        let mut list = ResourceList::default();
        let h1 = list.push(1u32);
        let h2 = list.push(2u32);

        // Construct an invalid handle that was never inserted
        let invalid = Handle::<u32>::default();
        list.release(invalid);
        // state should be unchanged
        assert_eq!(list.len(), 2);

        list.release(h1);
        assert_eq!(list.len(), 1);
        assert_eq!(*list.get_ref(h2), 2);
    }

    #[test]
    #[should_panic]
    fn zero_capacity_list() {
        let _ = ResourceList::<u32>::new(0);
    }
}
