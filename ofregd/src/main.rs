mod config;
mod handle;
mod query;
mod store;
mod ofreg {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/ofreg.skel.rs"
    ));
}

use std::{ffi::CString, mem::MaybeUninit, time::Duration};

use libbpf_rs::{
    RingBufferBuilder,
    skel::{OpenSkel, Skel, SkelBuilder},
};
use tracing::{error, info};
use tracing_appender::{non_blocking::WorkerGuard, rolling};

use ofreg_common::DB_PATH;

use handle::handle;
use ofreg::*;
use query::QuerySrv;
unsafe impl plain::Plain for types::commit {}

const MAX_PATH_LEN: usize = 256;

fn init_log() -> WorkerGuard {
    let file_appender = rolling::daily("logs", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false) // 输出到文件
        .init();

    info!("ofregd start...");
    guard
}

fn ebpf_load_run(target_dir: &str) {
    let skel_builder = OfregSkelBuilder::default();
    let mut open_object = MaybeUninit::uninit();
    let open_skel = skel_builder.open(&mut open_object).expect("bpf init error");

    // let target_dir = unsafe { CStr::from_ptr(target_dir.as_ptr() as *const i8) };
    let target_dir = CString::new(target_dir).expect("target dir can't convert to cstr");
    let target_dir_bytes = target_dir.as_bytes_with_nul();
    let mut target_dir_c = [0i8; MAX_PATH_LEN];
    target_dir_bytes
        .iter()
        .enumerate()
        .for_each(|(i, b)| target_dir_c[i] = *b as i8);

    open_skel.maps.rodata_data.target_dir = target_dir_c;

    let mut skel = open_skel.load().expect("bpf init error");

    let mut commit_builder = RingBufferBuilder::new();
    commit_builder
        .add(&skel.maps.shuttle, |data| handle(data))
        .expect("failed to bind ringbuf");

    let commit = commit_builder.build().expect("failed to build ringbuf");

    skel.attach().expect("bpf init error");

    loop {
        let n = commit.poll_raw(Duration::MAX);
        if n < 0 {
            break;
        }
    }
}

fn db_file_init() {
    let db_path = std::path::Path::new(DB_PATH);
    if !db_path.exists() {
        std::fs::create_dir_all(db_path).expect("create db path error");
        info!("create db path");
    }
}

fn main() {
    let _guard = init_log();

    db_file_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| error!("{e}"))
        .unwrap();
    rt.spawn(async {
        let query_srv = QuerySrv::new_conn().await.expect("query srv fail create");
        if let Err(e) = query_srv.srv().await {
            panic!("{}", e);
        }
    });

    let args = std::env::args();
    if args.len() != 2 {
        panic!("usage: ofreg <config path>");
    }

    let config_path = args.last().unwrap();
    config::init_config(config_path).expect("config init error");

    let target_dir = &config::CONFIG.get().expect("unreachable").target_dir;

    if target_dir.len() >= MAX_PATH_LEN {
        panic!("path too long");
    }

    ebpf_load_run(target_dir);
}
