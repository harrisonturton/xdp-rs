load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

cargo_build_script(
    name = "build_script",
    edition = "2021",
    srcs = ["build.rs"],
    deps = all_crate_deps(
      build = True,
    ),
    data = glob(["include/**/*"]),
)

rust_library(
    name = "xdp-sys",
    edition = "2021",
    srcs = glob([
        "src/**/*.rs",
    ]),
    deps = [":build_script"],
    visibility = ["//visibility:public"],
)