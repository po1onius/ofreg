#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>
#include <string.h>

#define MAX_PATH_LEN 128

struct commit {
    int pid;
    u64 exec_ts;
    char exe_file_path[MAX_PATH_LEN];
    char op_file_path[MAX_PATH_LEN];
};

// 定义全局变量过滤目录路径
const volatile char target_dir[MAX_PATH_LEN] = "/home/srus"; // 修改为目标目录

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024);
} shuttle SEC(".maps");

struct commit _export = {};

SEC("fentry/__x64_sys_openat")
int BPF_PROG(open_file_fentry, struct pt_regs *regs) {
    char op_file_path_buf[MAX_PATH_LEN] = {};
    bpf_core_read_user(op_file_path_buf, MAX_PATH_LEN, PT_REGS_PARM2(regs));

    if (strstr(op_file_path_buf, target_dir) != op_file_path_buf) {
        return 0;
    }

    struct task_struct *task = (struct task_struct *)(bpf_get_current_task_btf());

    struct path f_path = BPF_CORE_READ(task, mm, exe_file, f_path);
    struct qstr d_name = BPF_CORE_READ(&f_path, dentry, d_name);
    const unsigned char *name = BPF_CORE_READ(&d_name, name);


    struct commit *commit = bpf_ringbuf_reserve(&shuttle, sizeof(struct commit), 0);
    if (!commit) {
        return 0;
    }
    commit->pid = bpf_get_current_pid_tgid() >> 32;
    commit->exec_ts = bpf_ktime_get_ns();
    __builtin_memcpy(commit->op_file_path, op_file_path_buf, MAX_PATH_LEN);
    bpf_core_read_str(commit->exe_file_path, MAX_PATH_LEN, name);
    bpf_ringbuf_submit(commit, 0);

    return 0;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";

