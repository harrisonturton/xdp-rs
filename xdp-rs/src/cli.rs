use crate::sys::mmap::{Behavior, Protection};
use crate::{constants, sys, umem, Result};

/// list for packets with xdp
#[derive(argh::FromArgs)]
struct Args {
    /// network device name
    #[argh(option)]
    ifname: String,
    /// index of the netdev rx/tx queue
    #[argh(option)]
    queue_id: u32,
}

pub fn exec() -> Result<()> {
    let _args = argh::from_env::<Args>();

    println!("Attempting to mmap");
    let packet_buf = sys::mmap::private()
        .length(constants::PACKET_BUFFER_SIZE)
        .behaviour(Behavior::Anonymous)
        .protection(Protection::Read | Protection::Write | Protection::Exec)
        .build()?;
    println!("Successfully mmapped");

    println!("Creating socket");
    let sock = sys::socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;
    println!("Successfully created socket");

    println!("Creating umem");
    let umem = umem::Umem::create(sock, packet_buf)?;
    println!("Created umem: {:?}", umem);

    // let sock = sys::socket::create(AF_XDP, SOCK_RAW, 0)?;
    // sock.set_opt(SOL_XDP, xdp_sys::XDP_UMEM_REG, todo!())?;

    // let sock = sys::socket(AF_XDP, SOCK_RAW, 0)?;
    // sys::setsockopt(sock, SOL_XDP, xsp_sys::XDP_UMEM_REG, reg)?;

    Ok(())
}
