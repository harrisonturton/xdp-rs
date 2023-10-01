use crate::sys::{
    mmap::{Behavior, Mmap, Protection, Visibility},
    socket::{Socket, XdpMmapOffsets},
};
use crate::Result;
use std::{marker::PhantomData, mem::size_of};

#[must_use]
pub fn new_fill_ring(socket: &Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &size)?;
    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = (offsets.fr.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.fr.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.fr.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = mmap.addr.as_ptr().offset(offsets.fr.desc as isize);
        addr as *mut u64
    };

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

#[must_use]
pub fn new_completion_ring(socket: &Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_COMPLETION_RING, &size)?;
    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = (offsets.cr.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.cr.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.cr.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = mmap.addr.as_ptr().offset(offsets.cr.desc as isize);
        addr as *mut u64
    };

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

#[must_use]
pub fn new_rx_ring(socket: &Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_RX_RING, &size)?;
    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = (offsets.rx.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.rx.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.rx.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = mmap.addr.as_ptr().offset(offsets.rx.desc as isize);
        addr as *mut xdp_sys::xdp_desc
    };

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

#[must_use]
pub fn new_tx_ring(socket: &Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_TX_RING, &size)?;
    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = (offsets.tx.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_TX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.tx.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.tx.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = mmap.addr.as_ptr().offset(offsets.tx.desc as isize);
        addr as *mut xdp_sys::xdp_desc
    };

    Ok(RingBuffer::new(size, producer, consumer, descs))
}

/// Safe wrapper for interacting with the fill, completion, RX and TX rings
/// attached to the UMEM and AF_XDP sockets.
///
/// The `producer` and `consumer` fields are pointers to the mmapped `struct
/// xdp_ring` kernel struct. This is why the lifetime 'a is tied to an Mmap
/// instance, because once that Mmap is dropped (and assumed unmapped) the
/// pointers will become invalid.
#[derive(Debug, PartialEq, Eq)]
pub struct RingBuffer<'a, T> {
    cap: usize,
    // Producer and consumer indices increment unbounded, and wrap around normally.
    producer: *mut u32,
    consumer: *mut u32,
    descs: *mut T,
    map: PhantomData<&'a Mmap>,
}

impl<'a, T> RingBuffer<'a, T> {
    pub fn new(
        cap: usize,
        producer: *mut u32,
        consumer: *mut u32,
        descs: *mut T,
    ) -> RingBuffer<'a, T> {
        assert!(cap % 2 == 0, "capacity must be a power of two");
        RingBuffer {
            cap,
            producer,
            consumer,
            descs,
            map: PhantomData,
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
    fn new_test_buffer<'a, T>(cap: usize) -> RingBuffer<'a, T> {
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
                map: PhantomData,
            }
        }
    }
}
