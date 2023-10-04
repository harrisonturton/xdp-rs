#![feature(ip_bits)]

use byteorder::{NetworkEndian, ReadBytesExt};
use std::error::Error;
use std::io::Cursor;
use std::net::Ipv6Addr;
use xdp::constants::{
    DEFAULT_CONS_NUM_DESCS, DEFAULT_FRAME_HEADROOM, DEFAULT_PROD_NUM_DESCS, FRAME_SIZE, NUM_FRAMES,
};
use xdp::socket::XdpSocket;
use xdp::sys::if_nametoindex;
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

    let mut umem = Umem::builder()
        .frame_count(NUM_FRAMES)
        .frame_size(FRAME_SIZE)
        .frame_headroom(DEFAULT_FRAME_HEADROOM as u32)
        .build()?;

    let mut xsk = XdpSocket::builder()
        .owned_umem(&umem)
        .rx_size(DEFAULT_PROD_NUM_DESCS)
        .tx_size(DEFAULT_CONS_NUM_DESCS)
        .build()?;

    xsk.bind(ifindex, args.queue_id)?;

    let obj_buf = std::fs::read(&args.filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;

    let prog = obj.find_program(&args.program)?;
    prog.attach_xdp(ifindex)?;

    let key = u32::to_le_bytes(0);
    let value = u32::to_le_bytes(xsk.fd());
    obj.find_map("xsks_map")?.update(&key, &value)?;

    let (mut fr, _cr) = umem.rings();
    let (mut rx, _tx) = xsk.rings();

    for i in 0..fr.capacity() {
        fr.enqueue(i as u64);
    }

    println!("Polling...");

    loop {
        if xdp::sys::poll(xsk.fd(), libc::POLLIN) != 1 {
            println!("Skipping poll");
            continue;
        }

        for _ in 0..rx.len() {
            let desc = rx.dequeue().unwrap();

            let bytes = umem.frame(desc.addr);
            let mut rdr = Cursor::new(bytes);

            // Quick and dirty IPV6 header parsing
            let (src, dest) = {
                rdr.read_u32::<NetworkEndian>()
                    .expect("could not read packet bytes");
                rdr.read_u32::<NetworkEndian>()
                    .expect("could not read packet bytes");

                let src = rdr.read_u128::<NetworkEndian>().expect("bad ipv6 src");
                let dest = rdr.read_u128::<NetworkEndian>().expect("bad ipv6 dest");

                (Ipv6Addr::from_bits(src), Ipv6Addr::from_bits(dest))
            };

            println!("Got packet: len={} src={src:?} dest={dest:?}", desc.len);

            fr.enqueue(desc.addr);
        }
    }
}
