load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "ipv6-logger",
    edition = "2021",
    srcs = glob([
        "src/**/*.rs",
    ]),
    aliases = aliases(),
    deps = [
      "//crates/bpf",
      "//crates/xdp",
      "//crates/xdp-sys",
      "@libelf",
      "@libbpf",
      "@libz",
    ] + all_crate_deps(),
    visibility = ["//visibility:public"],
)