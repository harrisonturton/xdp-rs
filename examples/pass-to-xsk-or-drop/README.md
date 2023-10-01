# xdp

This is a minimal example of using BPF and `AF_XDP` sockets to pass packets
directly from the NIC to userspace, with zero copies.

Currently only the receiving path is implemented, and it only receives packets
from the first queue (index 0) on the NIC.

## Dependencies

* `libbpf-dev`
* `libxdp-dev`
* `xdp-tools` (optional)

## Usage

First, load the BPF program (the `kernel` object file) into the kernel,
register it with a network device, and run the program:

```
% make
...
% cargo run -- bpf-loader kernel_prog.o xdp_pass_to_xsk <ifindex>
...
```

In a separate window, `ping` the network device. The `user` program will log
every packet, while `ping` reports 100% packet loss. This is because all the
packets are being sent directly to userspace, but are not responded to.

## References

* [libxdp documentation](https://www.mankier.com/3/libxdp)
* [The Path to DPDK Speeds for AF XDP](http://vger.kernel.org/lpc_net2018_talks/lpc18_pres_af_xdp_perf-v3.pdf)
* [Acclerating Networking With AF_XDP (LWN)](https://lwn.net/Articles/750845/)
* [AF_XDP address family documentation (Linux kernel)](https://www.kernel.org/doc/html/next/networking/af_xdp.html)
* [XDP tutorial repository (XDP project)](https://github.com/xdp-project/xdp-tutorial)
* [Kernel tree BPF samples (also contains XDP examples)](https://github.com/torvalds/linux/tree/master/samples/bpf)