use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
    time::Duration,
};

use anyhow::{Error, Result};
use libbpf_rs::{
    RingBufferBuilder,
    skel::{OpenSkel, Skel, SkelBuilder},
};

mod db;

mod ofreg {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/ofreg.skel.rs"
    ));
}

use ofreg::*;
unsafe impl plain::Plain for types::commit {}

mod distro;
mod handle;
use handle::handle;

fn main() -> Result<()> {
    let args = std::env::args();
    if args.len() != 2 {
        println!("usage: ofreg <path>");
        return Err(Error::msg("error arg"));
    }

    let target_dir = args.last().unwrap();

    let skel_builder = OfregSkelBuilder::default();
    let mut open_object = MaybeUninit::uninit();
    let open_skel = skel_builder.open(&mut open_object)?;

    let target_dir = unsafe { CStr::from_ptr(target_dir.as_ptr() as *const i8) };

    open_skel.maps.rodata_data.target_dir = unsafe { *(target_dir.as_ptr() as *const [i8; 128]) };

    let mut skel = open_skel.load()?;

    let mut commit_builder = RingBufferBuilder::new();
    commit_builder
        .add(&skel.maps.shuttle, |data| handle(data))
        .expect("failed to bind ringbuf");

    let commit = commit_builder.build().expect("failed to build ringbuf");

    skel.attach()?;

    loop {
        let n = commit.poll_raw(Duration::MAX);
        if n < 0 {
            break;
        }
    }
    Ok(())
}
