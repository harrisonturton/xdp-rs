use std::mem::size_of;
use crate::ring::RingBuffer;
use crate::sys::mmap::{self, Visibility, Behavior, Protection};
use crate::sys::socket::{Socket, XdpMmapOffsets};
use crate::umem::{Umem, UmemConfig};
use crate::{constants, sys, Result};

/// list for packets with xdp
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// network device name
    #[argh(option)]
    ifname: String,
    /// index of the netdev rx/tx queue
    #[argh(option)]
    queue_id: u32,
}

pub fn exec() -> Result<()> {
    let Args { ifname, queue_id } = argh::from_env::<Args>();
    println!("Listening on queue {queue_id} of interface {ifname}");

    let mut socket = sys::socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

    let _umem = Umem::create(UmemConfig {
        socket: &mut socket,
        frame_count: constants::NUM_FRAMES as u32,
        frame_size: constants::FRAME_SIZE as u32,
        frame_headroom: constants::DEFAULT_FRAME_HEADROOM as u32,
    })?;

    let mut fill_queue = new_fill_ring(&mut socket, constants::DEFAULT_PROD_NUM_DESCS as usize)?;
    let mut _comp_queue = new_completion_ring(&mut socket, constants::DEFAULT_CONS_NUM_DESCS as usize)?;
    let mut _rx_queue = new_rx_ring(&mut socket, constants::DEFAULT_CONS_NUM_DESCS as usize)?;

    let ifindex = sys::if_nametoindex(ifname)?;

    socket.bind(&xdp_sys::sockaddr_xdp {
        sxdp_family: libc::PF_XDP as u16,
        sxdp_flags: 0,
        sxdp_ifindex: ifindex,
        sxdp_queue_id: queue_id,
        sxdp_shared_umem_fd: 0,
    })?;

    println!("Bound successfully");

    // Need to add items to the fill queue so that packets can start being received
    println!("About to enqueue");
    for i in 0..fill_queue.len() {
        println!("Attempting enqueue {i}");
        fill_queue.enqueue(i as u64);
    }

    let mut pollfd = libc::pollfd {
        fd: socket.fd,
        events: libc::POLLIN,
        revents: 0,
    };

    loop {
        println!("Polling...");
        let ret = unsafe { libc::poll(&mut pollfd, 1, -1) };
        if ret <= 0 || ret > 1 {
            println!("Nothing returned from poll");
            continue;
        }

        println!("Packets received");

        // TODO: pop descriptors from RX
        // TODO: push those descriptors to fill queue to re-use
    }
}

#[must_use]
fn new_fill_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket
        .set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;
    let len = offsets.fr.desc + size as u64 * size_of::<u64>() as u64;
    let fill_ring_map = mmap::builder()
        .fd(socket.fd)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    Ok(unsafe {
        crate::ring::RingBuffer::new(fill_ring_map.addr, size)
    })
}

#[must_use]
fn new_completion_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket
        .set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_COMPLETION_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;
    let len = offsets.cr.desc + size as u64 * size_of::<u64>() as u64;
    let comp_ring_map = mmap::builder()
        .fd(socket.fd)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    Ok(unsafe {
        RingBuffer::new(comp_ring_map.addr, size)
    })
}

#[must_use]
fn new_rx_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
    socket
        .set_opt(libc::SOL_XDP, xdp_sys::XDP_RX_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;
    let len = offsets.rx.desc + (size as u64 * size_of::<xdp_sys::xdp_desc>() as u64);
    let rx_ring_map = mmap::builder()
        .fd(socket.fd)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    Ok(unsafe {
        RingBuffer::new(rx_ring_map.addr, size)
    })
}

#[must_use]
fn new_tx_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
        socket
            .set_opt(libc::SOL_XDP, xdp_sys::XDP_TX_RING, &size)?;

        let offsets = socket.get_opt::<XdpMmapOffsets>()?;
        let len = offsets.tx.desc + (size as u64 * size_of::<xdp_sys::xdp_desc>() as u64);
        let tx_ring_map = mmap::builder()
            .fd(socket.fd)
            .visibility(Visibility::Shared)
            .length(len as usize)
            .offset(xdp_sys::XDP_PGOFF_TX_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

    Ok(unsafe {
        RingBuffer::new(tx_ring_map.addr, size)
    })
}