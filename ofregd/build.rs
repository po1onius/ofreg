use libbpf_cargo::SkeletonBuilder;
use std::ffi::OsStr;

const SRC_C: &str = "src/bpf/ofreg.bpf.c";
const SRC_RS: &str = "src/bpf/ofreg.skel.rs";
const VMLINUX_H_PATH: &str = "src/bpf";

fn main() {
    SkeletonBuilder::new()
        .source(SRC_C)
        .clang_args([OsStr::new("-I"), OsStr::new(VMLINUX_H_PATH)])
        .build_and_generate(SRC_RS)
        .unwrap();
    println!("cargo:rerun-if-changed={SRC_C}");
}
