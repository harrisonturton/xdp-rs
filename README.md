# xdp-rs

Experimental library for using `AF_XDP` sockets in Rust.

## Libraries

* `/crates/xdp` main library for using `AF_XDP` sockets
* `/crates/xdp-sys` generated bindings for the XDP kernel headers
* `/crates/bpf` safe wrappers over `libbpf-sys` (generated libbpf bindings)

## Requirements

This mainly depends on `libbpf`, and notably does *not* depend on `libxdp`. It
is built with Bazel but inherits some libraries from the system, due to `libbpf`
depending on `libelf` and `libz`. However, the `libbpf-sys` is configured to
link these statically, so these files are only required at build time:

* `/usr/lib/x86_64-linux-gnu/libbpf.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libz.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libzstd.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libelf.{a,so}`

Those paths are defined in the `//third_party/*/*.BUILD` rules.

## Example

1. `cd examples/pass-to-xsk-or-drop && make` 
2. Create a network interface
3. Run `bazel run //examples/packet-counter ./examples/pass-to-xsk-or-drop/kernel_prog.o <ifname> <queue_id>`
4. In a seperate tab, `ping` the network interface
5. You should see each packet being logged in the XDP program, but dropped from the ping

## Usage

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

// Fill the entire fill_ring to tell the kernel we're ready to begin receiving packets
for i in 0..DEFAULT_CONS_NUM_DESCS {
    fill_ring.enqueue(i as u64);
}

loop {
    let mut pollfd = libc::pollfd {
        fd: socket.fd,
        events: libc::POLLIN,
        revents: 0,
    };

    // Block until packets are received in the RX ring
    if unsafe { libc::poll(&mut pollfd, 1, -1) } != 1 {
        println!("Poll failed");
        continue;
    }

    // Iterate over every packet
    for i in 0..rx_ring.len() {
        let desc = rx_ring.dequeue().unwrap();
        println!("Received packet at offset {} in UMEM", desc.addr);

        // Once the packet has been processed, put the descriptor back in the
        // fill ring for the kernel to re-use that memory for another packet
        fill_ring.enqueue(desc.addr);
    }
}
```