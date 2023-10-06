use crate::error::Error;
use crate::ring::{new_rx_ring, new_tx_ring, RxRing, TxRing};
use crate::sys::socket::XdpMmapOffsets;
use crate::Result;
use crate::{sys::socket::Socket, umem::Umem};

#[derive(Debug)]
pub struct XdpSocket<U> {
    sock: Socket,
    umem_ref: U,
    rx: RxRing,
    tx: TxRing,
    ifindex: u32,
    queue: u32,
}

unsafe impl<U: Send> Send for XdpSocket<U> {}
unsafe impl<U: Sync> Sync for XdpSocket<U> {}

#[derive(Debug)]
pub struct OwnedUmem {
    umem: Umem,
}

#[derive(Debug)]
pub struct SharedUmem {
    sock: Socket,
}

impl XdpSocket<OwnedUmem> {
    #[must_use]
    pub fn create<'a>(
        umem_ref: OwnedUmem,
        rx_size: usize,
        tx_size: usize,
        ifindex: u32,
        queue: u32,
    ) -> Result<XdpSocket<OwnedUmem>> {
        let sock = umem_ref.umem.sock;
        let offsets = sock.get_opt::<XdpMmapOffsets>()?;
        let rx = new_rx_ring(&sock, &offsets, rx_size)?;
        let tx = new_tx_ring(&sock, &offsets, tx_size)?;
        Ok(XdpSocket {
            sock,
            umem_ref,
            rx,
            tx,
            ifindex,
            queue,
        })
    }

    #[inline]
    #[must_use]
    pub fn umem(&mut self) -> &mut Umem {
        &mut self.umem_ref.umem
    }

    #[must_use]
    pub fn bind(&self) -> Result<()> {
        self.sock.bind(&xdp_sys::sockaddr_xdp {
            sxdp_family: libc::PF_XDP as u16,
            sxdp_flags: 0,
            sxdp_ifindex: self.ifindex,
            sxdp_queue_id: self.queue,
            sxdp_shared_umem_fd: 0,
        })
    }
}

impl XdpSocket<SharedUmem> {
    #[must_use]
    pub fn create<'a>(
        umem_ref: SharedUmem,
        rx_size: usize,
        tx_size: usize,
        ifindex: u32,
        queue: u32,
    ) -> Result<XdpSocket<SharedUmem>> {
        let sock = Socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;
        let offsets = sock.get_opt::<XdpMmapOffsets>()?;
        let rx = new_rx_ring(&sock, &offsets, rx_size)?;
        let tx = new_tx_ring(&sock, &offsets, tx_size)?;
        Ok(XdpSocket {
            sock,
            umem_ref,
            rx,
            tx,
            ifindex,
            queue,
        })
    }

    #[must_use]
    pub fn bind(&self) -> Result<()> {
        self.sock.bind(&xdp_sys::sockaddr_xdp {
            sxdp_family: libc::PF_XDP as u16,
            sxdp_flags: xdp_sys::XDP_SHARED_UMEM as u16,
            sxdp_ifindex: self.ifindex,
            sxdp_queue_id: self.queue,
            sxdp_shared_umem_fd: self.umem_ref.sock.fd as u32,
        })
    }
}

impl<U> XdpSocket<U> {
    #[must_use]
    pub fn builder() -> XdpSocketBuilder<U> {
        XdpSocketBuilder::new()
    }

    #[must_use]
    pub fn rings(&mut self) -> (RxRing, TxRing) {
        (self.rx, self.tx)
    }

    #[inline]
    #[must_use]
    pub fn fd(&self) -> u32 {
        self.sock.fd as u32
    }

    #[inline]
    #[must_use]
    pub fn socket(&self) -> Socket {
        self.sock
    }
}

#[derive(Debug, Default)]
pub struct XdpSocketBuilder<U> {
    umem_ref: Option<U>,
    rx_size: Option<usize>,
    tx_size: Option<usize>,
    ifindex: Option<u32>,
    queue: Option<u32>,
}

impl<U> XdpSocketBuilder<U> {
    #[must_use]
    pub fn new() -> Self {
        XdpSocketBuilder {
            umem_ref: None,
            rx_size: None,
            tx_size: None,
            ifindex: None,
            queue: None,
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
    pub fn ifindex(mut self, ifindex: u32) -> Self {
        self.ifindex = Some(ifindex);
        self
    }

    #[must_use]
    pub fn queue(mut self, queue: u32) -> Self {
        self.queue = Some(queue);
        self
    }
}

impl XdpSocketBuilder<OwnedUmem> {
    #[must_use]
    pub fn owned_umem(mut self, umem: Umem) -> Self {
        self.umem_ref = Some(OwnedUmem { umem });
        self
    }

    #[must_use]
    pub fn build(self) -> Result<XdpSocket<OwnedUmem>> {
        let umem_ref = self
            .umem_ref
            .ok_or_else(|| Error::InvalidArgument("umem must be specified"))?;
        let rx_size = self
            .rx_size
            .ok_or_else(|| Error::InvalidArgument("rx_size must be specified"))?;
        let tx_size = self
            .tx_size
            .ok_or_else(|| Error::InvalidArgument("tx_size must be specified"))?;
        let ifindex = self
            .ifindex
            .ok_or_else(|| Error::InvalidArgument("ifindex must be specified"))?;
        let queue = self
            .queue
            .ok_or_else(|| Error::InvalidArgument("queue must be specified"))?;
        XdpSocket::<OwnedUmem>::create(umem_ref, rx_size, tx_size, ifindex, queue)
    }
}

impl XdpSocketBuilder<SharedUmem> {
    #[must_use]
    pub fn shared_umem(mut self, xsk: &XdpSocket<OwnedUmem>) -> Self {
        self.umem_ref = Some(SharedUmem { sock: xsk.socket() });
        self
    }

    #[must_use]
    pub fn build(self) -> Result<XdpSocket<SharedUmem>> {
        let umem_ref = self
            .umem_ref
            .ok_or_else(|| Error::InvalidArgument("umem must be specified"))?;
        let rx_size = self
            .rx_size
            .ok_or_else(|| Error::InvalidArgument("rx_size must be specified"))?;
        let tx_size = self
            .tx_size
            .ok_or_else(|| Error::InvalidArgument("tx_size must be specified"))?;
        let ifindex = self
            .ifindex
            .ok_or_else(|| Error::InvalidArgument("ifindex must be specified"))?;
        let queue = self
            .queue
            .ok_or_else(|| Error::InvalidArgument("queue must be specified"))?;
        XdpSocket::<SharedUmem>::create(umem_ref, rx_size, tx_size, ifindex, queue)
    }
}
