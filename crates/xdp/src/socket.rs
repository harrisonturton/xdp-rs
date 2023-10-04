use crate::error::Error;
use crate::ring::{RingBuffer, RxRing, TxRing};
use crate::sys::mmap::{Behavior, Mmap, Protection, Visibility};
use crate::sys::ptr_offset;
use crate::sys::socket::XdpMmapOffsets;
use crate::Result;
use crate::{sys::socket::Socket, umem::Umem};
use std::mem::size_of;

#[derive(Debug)]
pub struct XdpSocket {
    sock: Socket,
    rx: RxRing,
    tx: TxRing,
    umem: UmemRef,
}

#[derive(Debug)]
pub enum UmemRef {
    Owned(xdp_sys::xdp_umem_reg, Socket),
    Shared(Socket),
}

impl XdpSocket {
    #[must_use]
    pub fn builder() -> XdpSocketBuilder {
        XdpSocketBuilder::new()
    }

    #[must_use]
    pub fn create<'a>(umem: UmemRef, rx_size: usize, tx_size: usize) -> Result<XdpSocket> {
        if let UmemRef::Owned(ref reg, sock) = umem {
            sock.set_opt::<xdp_sys::xdp_umem_reg>(libc::SOL_XDP, xdp_sys::XDP_UMEM_REG, reg)?;
        }

        let sock = match umem {
            UmemRef::Owned(_, sock) => sock,
            UmemRef::Shared(sock) => sock,
        };

        let offsets = sock.get_opt::<XdpMmapOffsets>()?;
        let rx = register_rx_ring(&sock, &offsets, rx_size)?;
        let tx = register_tx_ring(&sock, &offsets, tx_size)?;
        Ok(XdpSocket { umem, sock, rx, tx })
    }

    #[must_use]
    pub fn rings(&mut self) -> (RxRing, TxRing) {
        (self.rx, self.tx)
    }

    #[must_use]
    pub fn bind(&self, ifindex: u32, queue_id: u32) -> Result<()> {
        self.sock.bind(&xdp_sys::sockaddr_xdp {
            sxdp_family: libc::PF_XDP as u16,
            sxdp_flags: 0,
            sxdp_ifindex: ifindex,
            sxdp_queue_id: queue_id,
            sxdp_shared_umem_fd: match self.umem {
                UmemRef::Owned(_, _) => 0,
                UmemRef::Shared(sock) => sock.fd as u32,
            },
        })
    }

    #[inline]
    #[must_use]
    pub fn fd(&self) -> u32 {
        self.sock.fd as u32
    }
}

fn register_rx_ring<'a>(
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

fn register_tx_ring(
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

#[derive(Debug, Default)]
pub struct XdpSocketBuilder {
    umem: Option<UmemRef>,
    rx_size: Option<usize>,
    tx_size: Option<usize>,
}

impl XdpSocketBuilder {
    #[must_use]
    pub fn new() -> Self {
        XdpSocketBuilder {
            umem: None,
            rx_size: None,
            tx_size: None,
        }
    }

    #[must_use]
    pub fn rx_size(mut self, rx_size: usize) -> Self {
        self.rx_size = Some(rx_size);
        self
    }

    #[must_use]
    pub fn tx_size(mut self, tx_size: usize) -> Self {
        self.tx_size = Some(tx_size);
        self
    }

    #[must_use]
    pub fn owned_umem(mut self, umem: &Umem) -> Self {
        let reg = xdp_sys::xdp_umem_reg {
            addr: umem.frame_buffer.addr.as_ptr().addr() as u64,
            len: umem.frame_buffer.len as u64,
            chunk_size: umem.frame_size,
            headroom: umem.frame_headroom,
            flags: 0,
        };
        self.umem = Some(UmemRef::Owned(reg, umem.sock));
        self
    }

    #[must_use]
    pub fn shared_umem(mut self, sock: &XdpSocket) -> Self {
        self.umem = Some(UmemRef::Shared(sock.sock));
        self
    }

    #[must_use]
    pub fn build(self) -> Result<XdpSocket> {
        let umem = self
            .umem
            .ok_or_else(|| Error::InvalidArgument("umem must be specified"))?;
        let rx_size = self
            .rx_size
            .ok_or_else(|| Error::InvalidArgument("rx_size must be specified"))?;
        let tx_size = self
            .tx_size
            .ok_or_else(|| Error::InvalidArgument("tx_size must be specified"))?;
        XdpSocket::create(umem, rx_size, tx_size)
    }
}
