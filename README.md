# xdp-rs

Experimental library for using `AF_XDP` sockets in Rust.

## Crates

* `/crates/xdp` main library for using `AF_XDP` sockets
* `/crates/xdp-sys` generated bindings for the XDP kernel headers
* `/crates/bpf` safe wrappers over `libbpf-sys` (generated libbpf bindings)

## Dependencies

Depends on `libbpf` and notably does *not* depend on `libxdp`.

`libbpf` is statically linked into the binary, but requires `libelf` and `libz`
to be available at build time. The `WORKSPACE` file imports these as local
repositories, which isn't very hermetic, but it works.

Specifically, it links against these objects:

* `/usr/lib/x86_64-linux-gnu/libbpf.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libz.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libzstd.{a,so}`
* `/usr/lib/x86_64-linux-gnu/libelf.{a,so}`

Those paths are defined in the `//third_party/*/*.BUILD` rules.

On Debian/Ubuntu, they can be installed with:

```
apt-get install build-essential pkgconf zlib1g-dev libelf-dev libbpf-dev
```

## Example

1. `cd examples/pass-to-xsk-or-drop && make` 
2. Create a network interface
3. Run `bazel run //examples/packet-counter ./examples/pass-to-xsk-or-drop/kernel_prog.o <ifname> <queue_id>`
4. In a seperate tab, `ping` the network interface
5. You should see each packet being logged in the XDP program, but dropped from the ping

## Usage

```rust
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
```