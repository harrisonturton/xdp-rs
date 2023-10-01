#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

/**
 * Map of queue index to AF_XDP sockets
 */
struct {
  __uint(type, BPF_MAP_TYPE_XSKMAP);
  __type(key, __u32);
  __type(value, __u32);
  __uint(max_entries, 64);
} xsks_map SEC(".maps");

/**
 * A NIC may have multiple rx/tx queues. This implementation uses the index of
 * the queue the packet arrived on to select the corresponding entry in xsk_map.
 * If there is no entry, then we pass the packet to the networking subsystem.
 *
 * In theory, this allows userspace to register multiple AF_XDP sockets to
 * parallelize packet ingress. But I haven't tested this because laptop NICs
 * only have one rx/tx ring queue.
 *
 * These queues are visible in sysfs (/sys/class/net/<ifname>/queues)
 */
SEC("xdp")
int xdp_try_pass_to_xsk(struct xdp_md* ctx) {
  int index = ctx->rx_queue_index;

  if (bpf_map_lookup_elem(&xsks_map, &index)) {
    return bpf_redirect_map(&xsks_map, index, 0);
  }

  return XDP_DROP;
}

char _license[] SEC("license") = "GPL";
