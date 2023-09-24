use xdp_sys::xdp_umem_reg;

#[derive(Debug, PartialEq, Eq)]
pub struct UmemConfig {
    fill_size: u32,
    comp_size: u32,
    frame_size: u32,
    frame_headroom: u32,
}

/// Wrapper for the `xdp_umem_reg` kernel type.
#[repr(transparent)]
pub struct UmemRegister(xdp_umem_reg);

impl UmemRegister {
    #[must_use]
    pub fn builder() -> UmemRegisterBuilder {
        UmemRegisterBuilder::default()
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct UmemRegisterBuilder {
    addr: Option<u64>,
    len: Option<u64>,
    chunk_size: Option<u32>,
    headroom: Option<u32>,
    flags: Option<u32>,
}

impl UmemRegisterBuilder {
    #[must_use]
    pub fn addr(mut self, addr: u64) -> Self {
        self.addr = Some(addr);
        self
    }

    #[must_use]
    pub fn build(self) -> Option<UmemRegister> {
        Some(UmemRegister(xdp_umem_reg {
            addr: self.addr?,
            len: self.len?,
            chunk_size: self.chunk_size?,
            headroom: self.headroom?,
            flags: self.flags?,
        }))
    }
}
