use std::{fs::File, io::Read};
use tracing::error;

use crate::store::{OFREG_DB, insert_item};
use crate::types::commit;
use ofreg_common::OfregData;

pub fn handle(data: &[u8]) -> i32 {
    let commit = plain::from_bytes::<commit>(data)
        .map_err(|_| error!("plain parse error"))
        .unwrap();
    fn char_slice_to_str(data: &[i8]) -> String {
        let cstr = unsafe { std::ffi::CStr::from_ptr(data.as_ptr()) };
        cstr.to_string_lossy().into()
    }
    let op_file_path = char_slice_to_str(&commit.op_file_path);
    let exe_file_path = char_slice_to_str(&commit.exe_file_path);
    println!(
        "pid: {}\nstart time: {}\nfilename: {}\ncommand: {}",
        commit.pid, commit.exec_ts, op_file_path, exe_file_path
    );

    let data = OfregData {
        cmd: exe_file_path,
        op_file: op_file_path,
        time: commit.exec_ts.to_string(),
    };
    let conn = OFREG_DB
        .lock()
        .map_err(|_| error!("sqlite write connect lock fetch error"))
        .unwrap();
    let _ = insert_item(&*conn, &data);

    0
}
