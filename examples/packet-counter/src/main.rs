use std::error::Error;
use xdp::socket::XdpSocket;
use xdp::sys::if_nametoindex;
use xdp::sys::socket::Socket;
use xdp::umem::Umem;

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
    let ifindex = if_nametoindex(args.ifname.clone())?;

    // Setup XDP

    let sock = Socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

    let mut umem = Umem::builder()
        .socket(&sock)
        .frame_count(xdp::constants::NUM_FRAMES as u32)
        .frame_size(xdp::constants::FRAME_SIZE as u32)
        .frame_headroom(xdp::constants::DEFAULT_FRAME_HEADROOM as u32)
        .build()?;

    let mut xsk = XdpSocket::builder()
        .socket(sock)
        .owned_umem(&umem)
        .rx_size(xdp::constants::DEFAULT_PROD_NUM_DESCS as usize)
        .tx_size(xdp::constants::DEFAULT_CONS_NUM_DESCS as usize)
        .build()?;

    xsk.bind(ifindex, args.queue_id)?;

    // Prepare XDP

    let fd = xsk.fd();
    let (fr, _) = umem.rings();
    let (rx, _) = xsk.rings();

    for i in 0..fr.capacity() {
        fr.enqueue(i as u64);
    }

    // Start receiving packets

    let obj = load_bpf_program(&args.filepath)?;
    let prog = obj.find_program(&args.program)?;
    prog.attach_xdp(ifindex)?;

    let map = obj.find_map("xsks_map")?;
    let key = u32::to_le_bytes(0);
    let value = u32::to_le_bytes(fd);
    map.update(&key, &value)?;

    loop {
        println!("Polling...");
        let mut pollfd = libc::pollfd {
            fd: fd as i32,
            events: libc::POLLIN,
            revents: 0,
        };

        if unsafe { libc::poll(&mut pollfd, 1, -1) } != 1 {
            println!("Skipping poll");
            continue;
        }

        println!("Received {} packets", rx.len());

        for i in 0..rx.len() {
            let desc = rx.dequeue().unwrap();
            println!("  [{i}]: {desc:?}");
            fr.enqueue(desc.addr);
        }
    }
}

fn load_bpf_program(filepath: &str) -> Result<bpf::Object, Box<dyn Error>> {
    let obj_buf = std::fs::read(filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;
    Ok(obj)
}
