use libc::SOL_XDP;

use crate::error::Error;
use crate::sys::mmap::{self, Behavior, Protection, Visibility};
use crate::sys::socket::XdpMmapOffsets;
use crate::sys::{mmap::MmapRegion, socket::Socket};
use crate::Result;
use std::marker::PhantomData;
use std::mem::size_of;
use std::sync::atomic;

#[derive(Debug)]
pub struct RingBuffer<T> {
    pub data: MmapRegion,
    pub size: u32,
    pub producer: *mut T,
    pub consumer: *mut T,
    pub _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct RingBufferConfig<'a> {
    pub socket: &'a mut Socket,
    // Number of descriptors in the buffer
    pub size: u32,
}

impl RingBuffer<u64> {
    pub fn enqueue(&mut self, item: u64) -> Result<()> {
        let free_entries = self.size - unsafe { (*self.producer) - (*self.consumer) } as u32;
        if free_entries == 0 {
            return Err(Error::Efault("queue is already full"));
        }

        // TODO: Why do we AND with self.size - 1?
        let offset = unsafe { *self.producer } & (self.size - 1) as u64;
        let item_ptr = self.data.addr.as_ptr().wrapping_add(offset as usize) as *mut _;
        unsafe { std::ptr::write(item_ptr, item) };

        // Write barrier
        atomic::fence(atomic::Ordering::Release);

        self.producer = self.producer.wrapping_add(1);

        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<u64> {
        let entries = unsafe { *self.producer - *self.consumer };
        if entries == 0 {
            return None;
        }

        let offset = unsafe { *self.consumer } & (self.size - 1) as u64;
        let item_ptr = self.data.addr.as_ptr().wrapping_add(offset as usize) as *mut _;
        let item = unsafe { std::ptr::read::<u64>(item_ptr) };

        // Read barrier
        atomic::fence(atomic::Ordering::Acquire);

        self.consumer = self.consumer.wrapping_add(1);
        Some(item)
    }

    pub fn create_fill_ring<'a>(config: RingBufferConfig<'a>) -> Result<RingBuffer<u64>> {
        config
            .socket
            .set_opt(SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &config.size)?;

        let offsets = config.socket.get_opt::<XdpMmapOffsets>()?;
        let len = offsets.fr.desc + config.size as u64 * size_of::<u64>() as u64;
        let fill_ring_map = mmap::builder()
            .fd(config.socket.fd)
            .visibility(Visibility::Shared)
            .length(len as usize)
            .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let producer = fill_ring_map.addr.as_ptr() as u64 + offsets.fr.producer;
        let consumer = fill_ring_map.addr.as_ptr() as u64 + offsets.fr.consumer;

        Ok(RingBuffer {
            data: fill_ring_map,
            size: config.size,
            producer: producer as *mut u64,
            consumer: consumer as *mut u64,
            _marker: PhantomData,
        })
    }

    pub fn create_completion_ring<'a>(config: RingBufferConfig<'a>) -> Result<RingBuffer<u64>> {
        config
            .socket
            .set_opt(SOL_XDP, xdp_sys::XDP_UMEM_COMPLETION_RING, &config.size)?;

        let offsets = config.socket.get_opt::<XdpMmapOffsets>()?;
        let len = offsets.cr.desc + config.size as u64 * size_of::<u64>() as u64;
        let comp_ring_map = mmap::builder()
            .fd(config.socket.fd)
            .visibility(Visibility::Shared)
            .length(len as usize)
            .offset(xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let producer = comp_ring_map.addr.as_ptr() as u64 + offsets.cr.producer;
        let consumer = comp_ring_map.addr.as_ptr() as u64 + offsets.cr.consumer;

        Ok(RingBuffer {
            data: comp_ring_map,
            size: config.size,
            producer: producer as *mut _,
            consumer: consumer as *mut _,
            _marker: PhantomData,
        })
    }
}

impl RingBuffer<xdp_sys::xdp_desc> {
    pub fn create_rx_ring<'a>(
        config: RingBufferConfig<'a>,
    ) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
        config
            .socket
            .set_opt(SOL_XDP, xdp_sys::XDP_RX_RING, &config.size)?;

        let offsets = config.socket.get_opt::<XdpMmapOffsets>()?;
        let len = offsets.rx.desc + config.size as u64 * size_of::<xdp_sys::xdp_desc>() as u64;
        let rx_ring_map = mmap::builder()
            .fd(config.socket.fd)
            .visibility(Visibility::Shared)
            .length(len as usize)
            .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let producer = rx_ring_map.addr.as_ptr() as u64 + offsets.rx.producer;
        let consumer = rx_ring_map.addr.as_ptr() as u64 + offsets.rx.consumer;

        Ok(RingBuffer {
            data: rx_ring_map,
            size: config.size,
            producer: producer as *mut _,
            consumer: consumer as *mut _,
            _marker: PhantomData,
        })
    }

    pub fn create_tx_ring<'a>(
        config: RingBufferConfig<'a>,
    ) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
        config
            .socket
            .set_opt(SOL_XDP, xdp_sys::XDP_TX_RING, &config.size)?;

        let offsets = config.socket.get_opt::<XdpMmapOffsets>()?;
        let len = offsets.tx.desc + config.size as u64 * size_of::<xdp_sys::xdp_desc>() as u64;
        let tx_ring_map = mmap::builder()
            .fd(config.socket.fd)
            .visibility(Visibility::Shared)
            .length(len as usize)
            .offset(xdp_sys::XDP_PGOFF_TX_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let producer = tx_ring_map.addr.as_ptr() as u64 + offsets.tx.producer;
        let consumer = tx_ring_map.addr.as_ptr() as u64 + offsets.tx.consumer;

        Ok(RingBuffer {
            data: tx_ring_map,
            size: config.size,
            producer: producer as *mut _,
            consumer: consumer as *mut _,
            _marker: PhantomData,
        })
    }
}
