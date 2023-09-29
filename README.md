# xdp-rs

Library that makes it easy for applications to use `AF_XDP` sockets. This allows
packets to be sent directly from the NIC to the application, skipping the Linux
networking stack.