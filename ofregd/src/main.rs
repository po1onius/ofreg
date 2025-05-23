mod handle;
mod query;
mod store;
mod ofreg {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/ofreg.skel.rs"
    ));
}

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
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::fmt;

use handle::handle;
use ofreg::*;
use query::QuerySrv;
unsafe impl plain::Plain for types::commit {}

fn main() -> Result<()> {
    let file_appender = rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking) // 输出到文件
        .init();

    info!("ofregd start...");

    let args = std::env::args();
    if args.len() != 2 {
        panic!("usage: ofreg <path>");
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.spawn(async {
        let query_srv = QuerySrv::new_conn().await;
        query_srv.srv().await;
    });

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
