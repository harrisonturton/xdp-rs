load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "bpf-loader",
    edition = "2021",
    srcs = glob([
        "src/**/*.rs",
    ]),
    aliases = aliases(),
    deps = [
      "//crates/bpf",
      "//crates/xdp",
      "@libelf",
      "@libbpf",
      "@libz",
    ] + all_crate_deps(),
    visibility = ["//visibility:public"],
)