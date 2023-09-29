use std::mem::size_of;
use std::ptr::{self, NonNull};

#[derive(Debug, PartialEq, Eq)]
pub struct RingBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    producer: usize,
    consumer: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new RingBuffer from an allocated region of memory.
    ///
    /// You probably don't want to use this constructor; there are safer
    /// alternatives.
    ///
    /// `buffer` is a pointer to the allocated region memory, and `cap` is the
    /// number of `T` instances that fit within this buffer. These values must
    /// be set correctly; bad things will happen if this is not true. For this
    /// reason, this constructor is marked as unsafe.
    #[must_use]
    pub unsafe fn new(buffer: NonNull<T>, cap: usize) -> RingBuffer<T> {
        assert!(size_of::<T>() != 0, "Cannot handle zero-sized types");
        RingBuffer {
            ptr: buffer,
            producer: 0,
            consumer: 0,
            len: 0,
            cap,
        }
    }

    #[inline]
    pub fn enqueue(&mut self, elem: T) -> bool {
        if self.len == self.cap {
            return false;
        }

        unsafe {
            let producer = self.ptr.as_ptr().add(self.producer);
            ptr::write(producer, elem);
        }

        self.producer = (self.producer + 1) % self.cap;
        self.len += 1;
        true
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let item = unsafe {
            let consumer = self.ptr.as_ptr().add(self.consumer);
            ptr::read(consumer)
        };

        self.consumer = (self.consumer + 1) % self.cap;
        self.len -= 1;
        Some(item)
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, Layout};

    #[test]
    fn test_enqueue_within_capacity() {
        let mut buffer = new_test_buffer::<u64>(10);

        let ret = buffer.enqueue(1);

        assert_eq!(true, ret);
        assert_eq!(1, buffer.len());
        assert_eq!(10, buffer.capacity());
        assert_eq!(0, buffer.consumer);
        assert_eq!(1, buffer.producer);
    }

    #[test]
    fn test_enqueue_then_dequeue_within_capacity() {
        let mut buffer = new_test_buffer::<u64>(10);
        buffer.enqueue(1);

        let ret = buffer.dequeue();

        assert_eq!(Some(1), ret);
        assert_eq!(0, buffer.len());
        assert_eq!(10, buffer.capacity());
        assert_eq!(1, buffer.consumer);
        assert_eq!(1, buffer.producer);
    }

    #[test]
    fn test_enqueue_when_full() {
        let mut buffer = new_test_buffer::<u64>(2);
        buffer.enqueue(1);
        buffer.enqueue(2);

        let ret = buffer.enqueue(2);

        assert_eq!(false, ret);
        assert_eq!(2, buffer.len());
        assert_eq!(2, buffer.capacity());
        assert_eq!(0, buffer.consumer);
        assert_eq!(0, buffer.producer);
    }

    #[test]
    fn test_enqueue_when_producer_rotates_past_end_of_buffer() {
        let mut buffer = new_test_buffer::<u64>(2);
        buffer.enqueue(1);
        buffer.enqueue(2);
        buffer.dequeue();

        let ret = buffer.enqueue(3);

        assert_eq!(true, ret);
        assert_eq!(2, buffer.len());
        assert_eq!(2, buffer.capacity());
        assert_eq!(1, buffer.consumer);
        assert_eq!(1, buffer.producer);
    }

    #[test]
    fn test_dequeue_when_consumer_rotates_past_end_of_buffer() {
        let mut buffer = new_test_buffer::<u64>(2);
        buffer.enqueue(1);
        buffer.enqueue(2);
        buffer.dequeue();
        buffer.enqueue(3);
        buffer.dequeue();

        let ret = buffer.dequeue();

        assert_eq!(Some(3), ret);
        assert_eq!(0, buffer.len());
        assert_eq!(2, buffer.capacity());
        assert_eq!(1, buffer.consumer);
        assert_eq!(1, buffer.producer);
    }


    // Do not use outside of a test context
    fn new_test_buffer<T>(cap: usize) -> RingBuffer<T> {
        unsafe {
            let layout = Layout::array::<T>(cap).expect("invalid memory layout");
            let ptr = alloc(layout).cast();
            let buffer = NonNull::new(ptr).expect("allocated buffer cannot be null");
            RingBuffer::new(buffer, cap)
        }
    }
}
