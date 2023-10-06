use crate::sys::{
    mmap::{Behavior, Mmap, Protection, Visibility},
    ptr_offset,
    socket::Socket,
};
use crate::Result;
use std::mem::size_of;

pub type FillRing = RingBuffer<u64>;
pub type CompRing = RingBuffer<u64>;

pub type RxRing = RingBuffer<xdp_sys::xdp_desc>;
pub type TxRing = RingBuffer<xdp_sys::xdp_desc>;

pub(crate) fn new_rx_ring<'a>(
    sock: &Socket,
    offsets: &xdp_sys::xdp_mmap_offsets,
    size: usize,
) -> Result<RxRing> {
    sock.set_opt(libc::SOL_XDP, xdp_sys::XDP_RX_RING, &size)?;

    let len = (offsets.rx.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(sock.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = ptr_offset(mmap.addr, offsets.rx.producer as usize);
    let consumer = ptr_offset(mmap.addr, offsets.rx.consumer as usize);
    let descs = ptr_offset(mmap.addr, offsets.rx.desc as usize);

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

pub(crate) fn new_tx_ring(
    sock: &Socket,
    offsets: &xdp_sys::xdp_mmap_offsets,
    size: usize,
) -> Result<TxRing> {
    sock.set_opt(libc::SOL_XDP, xdp_sys::XDP_TX_RING, &size)?;

    let len = (offsets.tx.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(sock.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_TX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = ptr_offset(mmap.addr, offsets.tx.producer as usize);
    let consumer = ptr_offset(mmap.addr, offsets.tx.consumer as usize);
    let descs = ptr_offset(mmap.addr, offsets.tx.desc as usize);

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

/// Safe wrapper for interacting with the fill, completion, RX and TX rings
/// attached to the UMEM and AF_XDP sockets.
///
/// The `producer` and `consumer` fields are pointers to the mmapped `struct
/// xdp_ring` kernel struct. This is why the lifetime 'a is tied to an Mmap
/// instance, because once that Mmap is dropped (and assumed unmapped) the
/// pointers will become invalid.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RingBuffer<T> {
    cap: usize,
    // Producer and consumer indices increment unbounded, and wrap around normally.
    producer: *mut u32,
    consumer: *mut u32,
    descs: *mut T,
}

impl<T> RingBuffer<T> {
    pub fn new(cap: usize, producer: *mut u32, consumer: *mut u32, descs: *mut T) -> RingBuffer<T> {
        assert!(cap % 2 == 0, "capacity must be a power of two");
        RingBuffer {
            cap,
            producer,
            consumer,
            descs,
        }
    }

    #[inline]
    pub fn enqueue(&mut self, item: T) -> bool {
        if self.available_slots() == 0 {
            return false;
        }

        unsafe {
            let index = *self.producer() as usize;
            self.item(index).write(item);
        }

        *self.producer() += 1;
        true
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        if self.used_slots() == 0 {
            return None;
        }

        let item = unsafe {
            let index = *self.consumer() as usize;
            self.item(index).read()
        };

        *self.consumer() += 1;
        Some(item)
    }

    #[inline]
    fn item(&mut self, index: usize) -> *mut T {
        unsafe { self.descs.offset((index & self.cap) as isize) }
    }

    #[inline]
    pub fn producer(&mut self) -> &mut u32 {
        unsafe { self.producer.as_mut().unwrap() }
    }

    #[inline]
    fn consumer(&mut self) -> &mut u32 {
        unsafe { self.consumer.as_mut().unwrap() }
    }

    #[inline]
    fn used_slots(&self) -> usize {
        (unsafe { *self.producer - *self.consumer }) as usize
    }

    #[inline]
    fn available_slots(&self) -> usize {
        self.cap - self.used_slots()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.used_slots()
    }

    #[inline]
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
        let mut buffer = new_test_buffer::<u64>(8);

        let ret = buffer.enqueue(1);

        assert_eq!(true, ret);
        assert_eq!(1, buffer.len());
        assert_eq!(8, buffer.capacity());
        assert_eq!(0, *buffer.consumer());
        assert_eq!(1, *buffer.producer());
    }

    #[test]
    fn test_enqueue_then_dequeue_within_capacity() {
        let mut buffer = new_test_buffer::<u64>(10);
        buffer.enqueue(1);

        let ret = buffer.dequeue();

        assert_eq!(Some(1), ret);
        assert_eq!(0, buffer.len());
        assert_eq!(10, buffer.capacity());
        assert_eq!(1, *buffer.consumer());
        assert_eq!(1, *buffer.producer());
    }

    #[test]
    fn test_enqueue_when_full() {
        println!("Running enqueue TEST");

        let mut buffer = new_test_buffer::<u64>(2);
        buffer.enqueue(1);
        buffer.enqueue(2);

        let ret = buffer.enqueue(3);

        assert_eq!(false, ret);
        assert_eq!(2, buffer.len());
        assert_eq!(2, buffer.capacity());
        assert_eq!(0, *buffer.consumer());
        assert_eq!(2, *buffer.producer());
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
        assert_eq!(1, *buffer.consumer());
        assert_eq!(3, *buffer.producer());
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
        assert_eq!(3, *buffer.consumer());
        assert_eq!(3, *buffer.producer());
    }

    // Do not use outside of a test context
    fn new_test_buffer<'a, T>(cap: usize) -> RingBuffer<T> {
        assert!(cap % 2 == 0, "capacity must be a power of two");
        unsafe {
            let layout = Layout::array::<T>(cap).expect("invalid memory layout");
            let descs = alloc(layout).cast();
            let producer = Box::new(0u32);
            let consumer = Box::new(0u32);
            RingBuffer {
                cap,
                consumer: Box::into_raw(consumer),
                producer: Box::into_raw(producer),
                descs,
            }
        }
    }
}
