# xdp-rs

Experimental library for using `AF_XDP` sockets in Rust.

It additionally provides safe wrappers for a subset of `libbpf` and `libc`.

## Usage

See the [ipv6 packet logger example](./examples/ipv6-logger).

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