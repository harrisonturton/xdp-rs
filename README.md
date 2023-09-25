# xdp-rs

Library for interacting with `AF_XDP` sockets from userspace. These sockets
allow userspace programs to send and receive packets via a eBPF map of XDP
sockets (XSKs). This allows us to pass packets from the kernel to userspace with
zero copies, enabling high-performance userspace networking that can outperform
the standard kernel networking stack.

This library does not offer any facilities for creating eBPF programs and
attaching them to network interfaces; it assumes that has already been done.