# xdp-rs

Library that makes it easy for applications to use `AF_XDP` sockets. The general
process looks like:

```rust
// Create a BPF object from the BPF ELF object file
let obj_buf = std::fs::read(filepath)?;
let obj = bpf::Object::create(&obj_buf)?;
obj.load()?;

// Find and attach the BPF program to the XDP hook
let prog = obj.find_program(program)?;
prog.attach_xdp(ifindex)?;

// Create the AF_XDP socket
let socket = Socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

// Create the shared memory region (uses mmap)
let umem = Umem::create(UmemConfig {
    socket: &socket,
    frame_count: NUM_FRAMES as u32,
    frame_size: FRAME_SIZE as u32,
    frame_headroom: DEFAULT_FRAME_HEADROOM as u32,
})?;

// Create the rings associated with the UMEM 
let fill_ring = xdp::ring::new_fill_ring(&socket, DEFAULT_CONS_NUM_DESCS as usize)?;
let comp_ring = xdp::ring::new_completion_ring(&socket, DEFAULT_PROD_NUM_DESCS as usize)?;

// Create the rings associated with the socket
let rx_ring = xdp::ring::new_rx_ring(&socket, DEFAULT_CONS_NUM_DESCS as usize)?;
let tx_ring = xdp::ring::new_tx_ring(&socket, DEFAULT_PROD_NUM_DESCS as usize)?;

// Bind the AF_XDP socket to a specific network interface and device queue
socket.bind(&xdp_sys::sockaddr_xdp {
    sxdp_family: libc::PF_XDP as u16,
    sxdp_flags: 0,
    sxdp_ifindex: IFINDEX,
    sxdp_queue_id: QUEUE_ID,
    sxdp_shared_umem_fd: 0,
})?;

// Add the socket's file descriptor to the BPF_MAP_TYPE_XSKMAP map used by the
// BPF program to redirect packets to a given AF_XDP socket
let map = obj.find_map("xsks_map")?;
let key = u32::to_le_bytes(0);
let value = u32::to_le_bytes(socket.fd as u32);
map.update(&key, &value)?;

loop {
    let mut pollfd = libc::pollfd {
        fd: socket.fd,
        events: libc::POLLIN,
        revents: 0,
    };

    if unsafe { libc::poll(&mut pollfd, 1, -1) } != 1 {
        println!("Poll failed");
        continue;
    }

    // Pop descriptors from rx_ring
    // Do something with them
    // Push descriptors to fill_ring when done
}
```

The `examples/packet-counter` example can be used to load the
`examples/pass-to-xsk-or-drop` BPF program and receive packets redirected from
it.