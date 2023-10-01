use std::mem::size_of;
use xdp::constants::FRAME_SIZE;
use xdp::ring::RingBuffer;
use xdp::sys::mmap::{self, Behavior, Protection, Visibility};
use xdp::sys::socket::{Socket, XdpMmapOffsets};
use xdp::umem::{Umem, UmemConfig};
use xdp::{constants, sys, Result};

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

    let socket = sys::socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

    let _umem = Umem::create(UmemConfig {
        socket: &socket,
        frame_count: constants::NUM_FRAMES as u32,
        frame_size: constants::FRAME_SIZE as u32,
        frame_headroom: constants::DEFAULT_FRAME_HEADROOM as u32,
    })?;

    println!("Creating fill ring");
    let mut fill_queue = new_fill_ring2(&socket, constants::DEFAULT_PROD_NUM_DESCS as usize)?;
    println!("Creating comp ring");
    let mut _comp_queue = new_completion_ring(&socket, constants::DEFAULT_CONS_NUM_DESCS as usize)?;
    println!("Creating RX ring");
    let mut _rx_queue = new_rx_ring2(&socket, constants::DEFAULT_CONS_NUM_DESCS as usize)?;

    let ifindex = sys::if_nametoindex(ifname.clone())?;

    println!("Binding to interface name: {ifname} index: {ifindex} queue: {queue_id}");

    socket.bind(&xdp_sys::sockaddr_xdp {
        sxdp_family: libc::PF_XDP as u16,
        sxdp_flags: 0,
        sxdp_ifindex: ifindex,
        sxdp_queue_id: queue_id,
        sxdp_shared_umem_fd: 0,
    })?;

    // Ok(())

    // Need to add items to the fill queue so that packets can start being received
    println!("About to enqueue");
    for i in 0..10 {
        println!("Attempting enqueue {i}");
        fill_queue.enqueue(i as u64 * FRAME_SIZE as u64);
        println!("{}", i as u64 * FRAME_SIZE as u64);
    }

    println!("Producer: {}", fill_queue.producer());

    println!("Socket: {}", socket.fd);

    loop {
        println!("Polling...");
        let mut pollfd = libc::pollfd {
            fd: socket.fd,
            events: libc::POLLIN,
            revents: 0,
        };
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
fn new_fill_ring2(socket: &Socket, size: usize) -> Result<crate::ring::RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;
    println!("Offsets: {offsets:?}");

    let len = (offsets.fr.desc + size as u64) * size_of::<u64>() as u64;
    println!("Allocating {len}");

    println!("About to mmap fill ring");
    let fill_ring_map = mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write) // THIS IS NOT WORKING PROPERLY!
        .build()?;
    println!("mmapped fill ring");

    println!("addr: {:?}", fill_ring_map.addr);

    let producer = {
        let addr = usize::from(fill_ring_map.addr.addr());
        let addr = addr + offsets.fr.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(fill_ring_map.addr.addr());
        let addr = addr + offsets.fr.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = fill_ring_map.addr.as_ptr().offset(offsets.fr.desc as isize);
        addr as *mut u64
    };

    let buffer = crate::ring::RingBuffer::new(size, producer, consumer, descs);
    Ok(buffer)
}

#[must_use]
fn new_fill_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &size)?;

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

    Ok(unsafe { crate::ring::RingBuffer::new(fill_ring_map.addr.cast(), size) })
}

#[must_use]
fn new_completion_ring(socket: &Socket, size: usize) -> Result<RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_COMPLETION_RING, &size)?;

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

    Ok(unsafe { RingBuffer::new(comp_ring_map.addr.cast(), size) })
}

#[must_use]
fn new_rx_ring(socket: &Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_RX_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = offsets.rx.desc + (size as u64 * size_of::<xdp_sys::xdp_desc>() as u64);
    println!(
        "rx.desc = {:?} rx.producer = {:?} rx.consumer = {:?}",
        offsets.rx.desc, offsets.rx.producer, offsets.rx.consumer,
    );
    let rx_ring_map = mmap::builder()
        .fd(socket.fd)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    Ok(unsafe { RingBuffer::new(rx_ring_map.addr.cast(), size) })
}

#[must_use]
fn new_rx_ring2(
    socket: &Socket,
    size: usize,
) -> Result<crate::ring::RingBuffer<xdp_sys::xdp_desc>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_RX_RING, &size)?;

    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = offsets.rx.desc + (size as u64 * size_of::<xdp_sys::xdp_desc>() as u64);
    println!(
        "rx.desc = {:?} rx.producer = {:?} rx.consumer = {:?}",
        offsets.rx.desc, offsets.rx.producer, offsets.rx.consumer,
    );
    let map = mmap::builder()
        .fd(socket.fd)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_PGOFF_RX_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(map.addr.addr());
        let addr = addr + offsets.rx.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(map.addr.addr());
        let addr = addr + offsets.rx.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = map.addr.as_ptr().offset(offsets.rx.desc as isize);
        addr as *mut xdp_sys::xdp_desc
    };

    Ok(crate::ring::RingBuffer::new(
        size, producer, consumer, descs,
    ))
}

#[must_use]
fn new_tx_ring(socket: &mut Socket, size: usize) -> Result<RingBuffer<xdp_sys::xdp_desc>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_TX_RING, &size)?;

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

    Ok(unsafe { RingBuffer::new(tx_ring_map.addr.cast(), size) })
}
