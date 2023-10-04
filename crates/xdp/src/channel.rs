use crate::umem::Umem;
use crate::Result;
use crate::{error::Error, socket::XdpSocket};

pub struct XdpChannel {
    owner: XdpSocket,
    peers: Vec<XdpSocket>,
}

impl XdpChannel {
    #[must_use]
    pub fn new(umem_config: UmemConfig, sock_config: SockConfig) -> Result<Self> {
        let umem = Umem::builder()
            .frame_count(umem_config.frame_count)
            .frame_size(umem_config.frame_size)
            .frame_headroom(umem_config.frame_headroom)
            .build()?;

        let owner = XdpSocket::builder()
            .owned_umem(umem)
            .rx_size(sock_config.rx_size)
            .tx_size(sock_config.tx_size)
            .build()?;

        let peers = (0..sock_config.socks - 1)
            .map(|_| {
                XdpSocket::builder()
                    .shared_umem(&owner)
                    .rx_size(sock_config.rx_size)
                    .tx_size(sock_config.tx_size)
                    .build()
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(XdpChannel { owner, peers })
    }

    #[must_use]
    pub fn builder() -> XdpChannelBuilder {
        XdpChannelBuilder::new()
    }

    #[must_use]
    pub fn socks(&mut self) -> (&mut XdpSocket, std::slice::IterMut<'_, XdpSocket>) {
        (&mut self.owner, self.peers.iter_mut())
    }
}

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct XdpChannelBuilder {
    umem: Option<UmemConfig>,
    socks: Option<SockConfig>,
}

impl XdpChannelBuilder {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn umem(mut self, umem: UmemConfig) -> Self {
        self.umem = Some(umem);
        self
    }

    #[must_use]
    pub fn sockets(mut self, socks: SockConfig) -> Self {
        self.socks = Some(socks);
        self
    }

    #[must_use]
    pub fn build(self) -> Result<XdpChannel> {
        let umem = self
            .umem
            .ok_or(Error::NotFound("umem config is required"))?;
        let socks = self
            .socks
            .ok_or(Error::NotFound("sock config is required"))?;
        XdpChannel::new(umem, socks)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct UmemConfig {
    frame_count: u32,
    frame_size: u32,
    frame_headroom: u32,
}

impl UmemConfig {
    #[must_use]
    pub fn builder() -> UmemConfigBuilder {
        UmemConfigBuilder::new()
    }
}

impl Default for UmemConfig {
    fn default() -> Self {
        Self {
            frame_count: 4096,
            frame_size: 4096,
            frame_headroom: 0,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct UmemConfigBuilder {
    cfg: UmemConfig,
}

impl UmemConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn frame_count(mut self, frame_count: u32) -> Self {
        self.cfg.frame_count = frame_count;
        self
    }

    #[must_use]
    pub fn frame_size(mut self, frame_size: u32) -> Self {
        self.cfg.frame_size = frame_size;
        self
    }

    #[must_use]
    pub fn frame_headroom(mut self, frame_headroom: u32) -> Self {
        self.cfg.frame_headroom = frame_headroom;
        self
    }

    #[must_use]
    pub fn build(self) -> UmemConfig {
        self.cfg
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct SockConfig {
    socks: usize,
    rx_size: usize,
    tx_size: usize,
}

impl SockConfig {
    #[must_use]
    pub fn builder() -> SockConfigBuilder {
        SockConfigBuilder::new()
    }
}

impl Default for SockConfig {
    fn default() -> Self {
        Self {
            socks: 1,
            rx_size: 2048,
            tx_size: 2048,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
pub struct SockConfigBuilder {
    cfg: SockConfig,
}

impl SockConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn socks(mut self, socks: usize) -> Self {
        self.cfg.socks = socks;
        self
    }

    #[must_use]
    pub fn rx_size(mut self, rx_size: usize) -> Self {
        self.cfg.rx_size = rx_size;
        self
    }

    #[must_use]
    pub fn tx_size(mut self, tx_size: usize) -> Self {
        self.cfg.tx_size = tx_size;
        self
    }

    #[must_use]
    pub fn build(self) -> Result<SockConfig> {
        if self.cfg.socks == 0 {
            return Err(Error::InvalidArgument("must have at least one socket"));
        }
        Ok(self.cfg)
    }
}
