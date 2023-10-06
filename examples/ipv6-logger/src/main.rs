#![feature(ip_bits)]

use std::error::Error;
use xdp::channel::{DeviceConfig, SockConfig, UmemConfig, XdpChannel};
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

    let umem = UmemConfig::builder()
        .frame_count(NUM_FRAMES)
        .frame_size(FRAME_SIZE)
        .frame_headroom(DEFAULT_FRAME_HEADROOM as u32)
        .build();

    let socks = SockConfig::builder()
        .rx_size(DEFAULT_CONS_NUM_DESCS)
        .tx_size(DEFAULT_PROD_NUM_DESCS)
        .build();

    let netdev = DeviceConfig::builder()
        .queues([args.queue_id])
        .ifname(&args.ifname)
        .build()?;

    let mut chan = XdpChannel::builder()
        .umem(umem)
        .sockets(socks)
        .netdev(netdev)
        .build().expect("Could not create chan");

    let (owner, _) = chan.socks();
    owner.bind().expect("Could not bind");

    let (mut fr, mut _cr) = owner.umem().rings();
    let (mut rx, mut _tx) = owner.rings();

    let mut program = Program::from_file(&args.filepath, "pass_to_socket").expect("Could not create bpf prog");
    program.attach(ifindex).expect("could not attach");
    program.update_map("xsks_map", 0, owner.fd()).expect("could not update map");

    for i in 0..fr.capacity() {
        fr.enqueue(i as u64);
    }

    println!("Polling");

    loop {
        if xdp::sys::poll(owner.fd(), libc::POLLIN) != 1 {
            println!("Skipping poll");
            continue;
        }

        println!("Got {} packets", rx.len());

        for _ in 0..rx.len() {
            let desc = rx.dequeue().unwrap();
            println!("Got packet: len={}", desc.len);
            fr.enqueue(desc.addr);
        }
    }
}

pub struct Program {
    obj: bpf::Object,
    prog: bpf::Program,
}

impl Program {
    pub fn from_file(file: &str, name: &str) -> Result<Self, Box<dyn Error>> {
        let buf = std::fs::read(file)?;
        let obj = bpf::Object::create(&buf)?;
        obj.load()?;
        let prog = obj.find_program(name)?;
        Ok(Program { obj, prog })
    }

    pub fn attach(&mut self, ifindex: u32) -> Result<(), Box<dyn Error>> {
        self.prog.attach_xdp(ifindex)?;
        Ok(())
    }


    pub fn update_map<V: IntoMapValue>(&mut self, name: &str, key: V, value: V) -> Result<(), Box<dyn Error>> {
        let map = self.obj.find_map(name)?;
        map.update(&key.into_value(), &value.into_value())?;
        Ok(())
    }
}

pub trait IntoMapValue {
    fn into_value(self) -> Vec<u8>;
}

impl IntoMapValue for u32 {
    fn into_value(self) -> Vec<u8> {
        u32::to_le_bytes(self).to_vec()
    }
}