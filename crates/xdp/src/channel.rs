use std::slice::IterMut;

use crate::socket::{OwnedUmem, SharedUmem};
use crate::sys::if_nametoindex;
use crate::umem::Umem;
use crate::Result;
use crate::{error::Error, socket::XdpSocket};

pub struct XdpChannel {
    owner: XdpSocket<OwnedUmem>,
    peers: Vec<XdpSocket<SharedUmem>>,
}

impl XdpChannel {
    #[must_use]
    pub fn new(
        umem_config: UmemConfig,
        sock_config: SockConfig,
        device_config: DeviceConfig,
    ) -> Result<Self> {
        let ifindex = if_nametoindex(device_config.ifname)?;
        let mut queues = device_config.queues.iter();

        let umem = Umem::builder()
            .frame_count(umem_config.frame_count)
            .frame_size(umem_config.frame_size)
            .frame_headroom(umem_config.frame_headroom)
            .build()?;

        let owner_queue = queues
            .next()
            .ok_or_else(|| Error::InvalidArgument("must have at least one queue"))?
            .to_owned();

        let owner = XdpSocket::builder()
            .owned_umem(umem)
            .rx_size(sock_config.rx_size)
            .tx_size(sock_config.tx_size)
            .queue(owner_queue)
            .ifindex(ifindex)
            .build()?;

        let peers = queues
            .map(|queue| {
                XdpSocket::builder()
                    .shared_umem(&owner)
                    .rx_size(sock_config.rx_size)
                    .tx_size(sock_config.tx_size)
                    .queue(*queue)
                    .ifindex(ifindex)
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
    pub fn socks(
        &mut self,
    ) -> (
        &mut XdpSocket<OwnedUmem>,
        IterMut<'_, XdpSocket<SharedUmem>>,
    ) {
        (&mut self.owner, self.peers.iter_mut())
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct XdpChannelBuilder {
    umem: Option<UmemConfig>,
    socks: Option<SockConfig>,
    netdev: Option<DeviceConfig>,
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
    pub fn netdev(mut self, netdev: DeviceConfig) -> Self {
        self.netdev = Some(netdev);
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
        let netdev = self
            .netdev
            .ok_or(Error::NotFound("netdev config is required"))?;
        XdpChannel::new(umem, socks, netdev)
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
    pub fn build(self) -> SockConfig {
        self.cfg
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeviceConfig {
    queues: Vec<u32>,
    ifname: String,
}

impl DeviceConfig {
    #[inline]
    #[must_use]
    pub fn builder() -> DeviceConfigBuilder {
        DeviceConfigBuilder::new()
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct DeviceConfigBuilder {
    queues: Option<Vec<u32>>,
    ifname: Option<String>,
}

impl DeviceConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    #[must_use]
    pub fn queues<I>(mut self, queues: I) -> Self
    where
        I: IntoIterator<Item = u32>,
    {
        self.queues = Some(queues.into_iter().collect());
        self
    }

    #[must_use]
    pub fn ifname(mut self, name: &str) -> Self {
        self.ifname = Some(name.to_owned());
        self
    }

    #[must_use]
    pub fn build(self) -> Result<DeviceConfig> {
        let queues = self.queues.ok_or(Error::NotFound("queues are required"))?;
        let ifname = self.ifname.ok_or(Error::NotFound("ifname is required"))?;
        Ok(DeviceConfig { queues, ifname })
    }
}
