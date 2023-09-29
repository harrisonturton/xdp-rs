use crate::ring::{RingBuffer, RingBufferConfig};
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

    let mut fill_queue = RingBuffer::create_fill_ring(RingBufferConfig {
        socket: &mut socket,
        size: constants::DEFAULT_PROD_NUM_DESCS,
    })?;

    let _comp_queue = RingBuffer::create_completion_ring(RingBufferConfig {
        socket: &mut socket,
        size: constants::DEFAULT_CONS_NUM_DESCS,
    })?;

    let _rx_queue = RingBuffer::create_rx_ring(RingBufferConfig {
        socket: &mut socket,
        size: constants::DEFAULT_CONS_NUM_DESCS,
    })?;

    let ifindex = sys::if_nametoindex(ifname)?;

    socket.bind(&xdp_sys::sockaddr_xdp {
        sxdp_family: libc::PF_XDP as u16,
        sxdp_flags: 0,
        sxdp_ifindex: ifindex,
        sxdp_queue_id: queue_id,
        sxdp_shared_umem_fd: 0,
    })?;

    // Need to add items to the fill queue so that packets can start being received
    for i in 0..fill_queue.size - 1 {
        fill_queue.enqueue(i as u64)?;
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
