// https://liuhangbin.netlify.app/post/xdp_start/#task-1-xdp-drop-everything

// xdp_program_open__file(objfilename, ifname, NULL);
// xdp_program__attach(prog, ifindex, XDP_MODE_SKB, 0);
// xdp_program__bpf_obj(prog)
// bpf_object__find_map_fd_by_name(bpf_obj, "xsks_map")

// bpf_object__open_file
// Study "xdp_program__create_from_obj" https://github.com/xdp-project/xdp-tools/blob/master/lib/libxdp/libxdp.c#L1149
