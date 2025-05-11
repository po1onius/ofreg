use std::{fs::File, io::Read};

use crate::db::{DbOp, OFREG_DB, OfregData};
use crate::types::commit;

pub fn handle(data: &[u8]) -> i32 {
    let commit = plain::from_bytes::<commit>(data).unwrap();
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
    OFREG_DB.lock().unwrap().insert_item(&data);

    0
}
