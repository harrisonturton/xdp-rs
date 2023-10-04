load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "xdp",
    edition = "2021",
    srcs = glob([
        "src/**/*.rs",
    ]),
    aliases = aliases(),
    deps = [
        "//crates/xdp-sys",
        "//crates/bpf",
    ] + all_crate_deps(),
    visibility = ["//visibility:public"],
)