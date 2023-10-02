load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

http_archive(
    name = "rules_rust",
    sha256 = "c46bdafc582d9bd48a6f97000d05af4829f62d5fee10a2a3edddf2f3d9a232c1",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.28.0/rules_rust-v0.28.0.tar.gz"],
)

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")

rules_rust_dependencies()

rust_register_toolchains()

rust_register_toolchains(
    edition = "2021",
    versions = [
        "nightly/2023-09-29",
    ],
) 

load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")

crate_universe_dependencies()

load("@rules_rust//crate_universe:defs.bzl", "crates_repository", "crate")

crates_repository(
    name = "crate_index",
    cargo_lockfile = "//:Cargo.lock",
    lockfile = "//:cargo-bazel-lock.json",
    manifests = [
        "//:Cargo.toml",
        "//:crates/bpf/Cargo.toml",
        "//:crates/xdp/Cargo.toml",
        "//:crates/xdp-sys/Cargo.toml",
        "//:examples/bpf-loader/Cargo.toml",
        "//:examples/packet-counter/Cargo.toml",
    ],
    annotations = {
        "libbpf-sys": [crate.annotation(
            gen_build_script = False,
        )],
    }
)

load("@crate_index//:defs.bzl", "crate_repositories")

crate_repositories()

new_local_repository(
    name = "libelf",
    path = "/usr",
    build_file = "//third_party/libelf:libelf.BUILD"
)

new_local_repository(
    name = "libbpf",
    path = "/usr",
    build_file = "//third_party/libbpf:libbpf.BUILD"
)

new_local_repository(
    name = "libz",
    path = "/usr",
    build_file = "//third_party/libz:libz.BUILD"
)

new_local_repository(
    name = "libzstd",
    path = "/usr",
    build_file = "//third_party/libzstd:libzstd.BUILD"
)