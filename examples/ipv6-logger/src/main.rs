#![feature(ip_bits)]

use std::error::Error;
use xdp::channel::{SockConfig, UmemConfig, XdpChannel};
use xdp::constants::{
    DEFAULT_CONS_NUM_DESCS, DEFAULT_FRAME_HEADROOM, DEFAULT_PROD_NUM_DESCS, FRAME_SIZE, NUM_FRAMES,
};
use xdp::sys::if_nametoindex;

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

#[derive(Debug)]
enum Msg {
    Fill(u64),
    Complete(u64),
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = argh::from_env::<Args>();
    let ifindex = if_nametoindex(args.ifname.clone())?;

    let umem = UmemConfig::builder()
        .frame_count(NUM_FRAMES)
        .frame_size(FRAME_SIZE)
        .frame_headroom(DEFAULT_FRAME_HEADROOM as u32)
        .build();

    let socks = SockConfig::builder()
        .socks(2)
        .rx_size(DEFAULT_CONS_NUM_DESCS)
        .tx_size(DEFAULT_PROD_NUM_DESCS)
        .build()?;

    let mut chan = XdpChannel::builder().umem(umem).sockets(socks).build()?;

    let obj_buf = std::fs::read(&args.filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;
    let prog = obj.find_program(&args.program)?;
    prog.attach_xdp(ifindex)?;

    let (owner, _) = chan.socks();

    let key = u32::to_le_bytes(0);
    let value = u32::to_le_bytes(owner.fd());
    obj.find_map("xsks_map")?.update(&key, &value)?;

    let (mut fr, _) = owner.umem().unwrap().rings();
    let (_, _) = owner.rings();

    for i in 0..fr.capacity() {
        fr.enqueue(i as u64);
    }

    let (mut rx, _tx) = owner.rings();
    owner.bind(ifindex, args.queue_id).expect("Could not bind sock");

    loop {
        println!("Polling");

        if xdp::sys::poll(owner.fd(), libc::POLLIN) != 1 {
            println!("Skipping poll");
            continue;
        }

        for _ in 0..rx.len() {
            let desc = rx.dequeue().unwrap();
            println!("Got packet: len={}", desc.len);
            fr.enqueue(desc.addr);
        }
    }
}