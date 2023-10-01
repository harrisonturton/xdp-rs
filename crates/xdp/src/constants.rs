// Values largely taken from libxdp

pub const NUM_FRAMES: usize = 4096;
pub const FRAME_SIZE: usize = 4096; // USK_UMEM__DEFAULT_FRAME_SIZE
pub const PACKET_BUFFER_SIZE: usize = NUM_FRAMES * FRAME_SIZE;

pub const RX_BATCH_SIZE: usize = 64;

pub const DEFAULT_PROD_NUM_DESCS: u32 = 2048;
pub const DEFAULT_CONS_NUM_DESCS: u32 = 2048;
pub const DEFAULT_FRAME_HEADROOM: u32 = 0;

pub const INVALID_UMEM_FRAME: u64 = u64::MAX;
