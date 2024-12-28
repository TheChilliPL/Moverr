use crate::file_size::FileSize;
use atomiq::compat::{Atomic, IntAtomic, SimpleAtomic};
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub struct AtomicFileSize<T: SimpleAtomic<Value = u64>> {
    value: T,
}

impl<T: SimpleAtomic<Value = u64>> From<FileSize> for AtomicFileSize<T> {
    fn from(value: FileSize) -> Self {
        Self {
            value: T::new(value.as_bytes()),
        }
    }
}

impl<T: SimpleAtomic<Value = u64>> SimpleAtomic for AtomicFileSize<T> {
    type Value = FileSize;

    fn load(&self, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.load(ordering))
    }

    fn store(&self, value: Self::Value, ordering: Ordering) {
        self.value.store(value.as_bytes(), ordering)
    }
}

impl<T: Atomic<Value = u64>> Atomic for AtomicFileSize<T> {
    fn compare_exchange(
        &self,
        current: Self::Value,
        new: Self::Value,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Value, Self::Value> {
        match self
            .value
            .compare_exchange(current.as_bytes(), new.as_bytes(), success, failure)
        {
            Ok(value) => Ok(FileSize::from_bytes(value)),
            Err(value) => Err(FileSize::from_bytes(value)),
        }
    }

    fn compare_exchange_weak(
        &self,
        current: Self::Value,
        new: Self::Value,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Value, Self::Value> {
        match self
            .value
            .compare_exchange_weak(current.as_bytes(), new.as_bytes(), success, failure)
        {
            Ok(value) => Ok(FileSize::from_bytes(value)),
            Err(value) => Err(FileSize::from_bytes(value)),
        }
    }

    fn fetch_update<F>(
        &self,
        set_ordering: Ordering,
        fetch_ordering: Ordering,
        mut f: F,
    ) -> Result<Self::Value, Self::Value>
    where
        F: FnMut(Self::Value) -> Option<Self::Value>,
    {
        let mut g = |value| f(FileSize::from_bytes(value)).map(|value| value.as_bytes());
        self.value
            .fetch_update(set_ordering, fetch_ordering, move |value| g(value))
            .map(FileSize::from_bytes)
            .map_err(FileSize::from_bytes)
    }

    fn swap(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.swap(value.as_bytes(), ordering))
    }
}

impl<T: IntAtomic<Value = u64>> IntAtomic for AtomicFileSize<T> {
    fn fetch_add(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_add(value.as_bytes(), ordering))
    }

    fn fetch_and(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_and(value.as_bytes(), ordering))
    }

    fn fetch_max(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_max(value.as_bytes(), ordering))
    }

    fn fetch_min(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_min(value.as_bytes(), ordering))
    }

    fn fetch_nand(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_nand(value.as_bytes(), ordering))
    }

    fn fetch_or(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_or(value.as_bytes(), ordering))
    }

    fn fetch_sub(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_sub(value.as_bytes(), ordering))
    }

    fn fetch_xor(&self, value: Self::Value, ordering: Ordering) -> Self::Value {
        FileSize::from_bytes(self.value.fetch_xor(value.as_bytes(), ordering))
    }
}
