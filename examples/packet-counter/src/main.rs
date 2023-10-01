use std::error::Error;
use xdp::constants::{DEFAULT_CONS_NUM_DESCS, DEFAULT_PROD_NUM_DESCS};
use xdp::umem::{Umem, UmemConfig};

/// count packets arriving on a given network interface
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// BPF program object file
    #[argh(positional)]
    filepath: String,
    /// name of the program
    #[argh(positional)]
    program: String,
    /// network interface name
    #[argh(positional)]
    ifname: String,
    /// network device queue ID
    #[argh(positional)]
    queue_id: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = argh::from_env::<Args>();

    let ifindex = xdp::sys::if_nametoindex(args.ifname.clone())?;
    println!(
        "Binding to interface name: {} index: {ifindex} queue: {}",
        args.ifname, args.queue_id
    );

    let (obj, _prog) = load_and_attach_bpf_program(&args.filepath, &args.program, ifindex)?;

    let socket = xdp::sys::socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

    let _umem = Umem::create(UmemConfig {
        socket: &socket,
        frame_count: xdp::constants::NUM_FRAMES as u32,
        frame_size: xdp::constants::FRAME_SIZE as u32,
        frame_headroom: xdp::constants::DEFAULT_FRAME_HEADROOM as u32,
    })?;

    let mut fill_ring = xdp::ring::new_fill_ring(&socket, DEFAULT_CONS_NUM_DESCS as usize)?;
    let _comp_ring = xdp::ring::new_completion_ring(&socket, DEFAULT_PROD_NUM_DESCS as usize)?;
    let mut rx_ring = xdp::ring::new_rx_ring(&socket, DEFAULT_CONS_NUM_DESCS as usize)?;
    let _tx_ring = xdp::ring::new_tx_ring(&socket, DEFAULT_PROD_NUM_DESCS as usize)?;

    for i in 0..10 {
        println!("Attempting enqueue {i}");
        fill_ring.enqueue(i as u64);
    }

    println!("setsockopt(ifindex={} queue_id={})", ifindex, args.queue_id);
    socket.bind(&xdp_sys::sockaddr_xdp {
        sxdp_family: libc::PF_XDP as u16,
        sxdp_flags: 0,
        sxdp_ifindex: ifindex,
        sxdp_queue_id: args.queue_id,
        sxdp_shared_umem_fd: 0,
    })?;

    println!("Updating xsks_map");
    let map = obj.find_map("xsks_map")?;
    let key = u32::to_le_bytes(0);
    let value = u32::to_le_bytes(socket.fd as u32);
    if let Err(bpf::Error::Errno(errno)) = map.update(&key, &value) {
        println!("map updated failed: {}", xdp::sys::strerror(errno));
        return Ok(());
    }
    println!("map update succeeded");

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

        println!("Received {} packets", rx_ring.len());
        for i in 0..rx_ring.len() {
            let desc = rx_ring.dequeue().unwrap();
            println!("[{i}]: {desc:?}");
            fill_ring.enqueue(desc.addr);
        }
    }
}

fn load_and_attach_bpf_program(
    filepath: &str,
    program: &str,
    ifindex: u32,
) -> Result<(bpf::Object, bpf::Program), Box<dyn Error>> {
    let obj_buf = std::fs::read(filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;

    let prog = obj.find_program(program)?;
    prog.attach_xdp(ifindex)?;

    Ok((obj, prog))
}
