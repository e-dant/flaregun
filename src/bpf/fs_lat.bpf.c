/* SPDX-License-Identifier: GPL-2.0 */
/* Copyright (c) 2020 Wenbo Zhang */
#include "constants.h"
#include "vmlinux.h"
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

#define MAX_ENTRIES 8192
#define FILE_NAME_LEN 32

volatile const __u64 min_lat_us = 0;
volatile const pid_t targ_tgid = 0;
// todo ^
volatile const pid_t targ_pid = 0;

#ifdef ALLOW_UNSAFE_ENUM
enum fs_file_op {
  F_READ,
  F_WRITE,
  F_OPEN,
  F_FSYNC,
  F_MAX_OP,
} _fs_file_op = {};
#else
static const u8 F_READ = 0;
static const u8 F_WRITE = 1;
static const u8 F_OPEN = 2;
static const u8 F_FSYNC = 3;
#endif

struct event {
  __u64 lat_us;
  __u64 end_ns;
  __s64 offset;
  ssize_t size;
  pid_t pid;
  u8 op;
  u8 file[FILE_NAME_LEN];
  u8 task[FL_TASK_COMM_LEN];
} _event = {};

struct data {
  __u64 ts;
  loff_t start;
  loff_t end;
  struct file* fp;
};

struct {
  __uint(type, BPF_MAP_TYPE_HASH);
  __uint(max_entries, MAX_ENTRIES);
  __type(key, __u32);
  __type(value, struct data);
} starts SEC(".maps");

struct {
  __uint(type, BPF_MAP_TYPE_PERF_EVENT_ARRAY);
  __uint(key_size, sizeof(__u32));
  __uint(value_size, sizeof(__u32));
} events SEC(".maps");

static int probe_entry(struct file* fp, loff_t start, loff_t end)
{
  __u64 pid_tgid = bpf_get_current_pid_tgid();
  __u32 pid = pid_tgid >> 32;
  __u32 tid = (__u32)pid_tgid;
  struct data data;

  if (! fp)
    return 0;

  if (targ_pid && targ_pid != pid)
    return 0;

  data.ts = bpf_ktime_get_ns();
  data.start = start;
  data.end = end;
  data.fp = fp;
  bpf_map_update_elem(&starts, &tid, &data, BPF_ANY);
  return 0;
}

static int probe_exit(void* ctx, u8 op, ssize_t size)
{
  __u64 pid_tgid = bpf_get_current_pid_tgid();
  __u32 pid = pid_tgid >> 32;
  __u32 tid = (__u32)pid_tgid;
  __u64 end_ns, delta_us;
  __u8 const* file_name;
  struct data* datap;
  struct event event = {};
  struct dentry* dentry;
  struct file* fp;

  if (targ_pid && targ_pid != pid)
    return 0;

  datap = bpf_map_lookup_elem(&starts, &tid);
  if (! datap)
    return 0;

  bpf_map_delete_elem(&starts, &tid);

  end_ns = bpf_ktime_get_ns();
  delta_us = (end_ns - datap->ts) / 1000;
  if (delta_us <= min_lat_us)
    return 0;

  event.lat_us = delta_us;
  event.end_ns = end_ns;
  event.offset = datap->start;
  if (op != F_FSYNC)
    event.size = size;
  else
    event.size = datap->end - datap->start;
  event.pid = pid;
  event.op = op;
  fp = datap->fp;
  dentry = BPF_CORE_READ(fp, f_path.dentry);
  file_name = BPF_CORE_READ(dentry, d_name.name);
  bpf_probe_read_kernel_str(&event.file, sizeof(event.file), file_name);
  bpf_get_current_comm(&event.task, sizeof(event.task));
  bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
  return 0;
}

SEC("kprobe/vfs_read")

int BPF_KPROBE(file_read_entry, struct kiocb* iocb)
{
  struct file* fp = BPF_CORE_READ(iocb, ki_filp);
  loff_t start = BPF_CORE_READ(iocb, ki_pos);

  return probe_entry(fp, start, 0);
}

SEC("kretprobe/vfs_read")

int BPF_KRETPROBE(file_read_exit, ssize_t ret) { return probe_exit(ctx, F_READ, ret); }

SEC("kprobe/vfs_write")

int BPF_KPROBE(file_write_entry, struct kiocb* iocb)
{
  struct file* fp = BPF_CORE_READ(iocb, ki_filp);
  loff_t start = BPF_CORE_READ(iocb, ki_pos);

  return probe_entry(fp, start, 0);
}

SEC("kretprobe/vfs_write")

int BPF_KRETPROBE(file_write_exit, ssize_t ret) { return probe_exit(ctx, F_WRITE, ret); }

SEC("kprobe/vfs_open")

int BPF_KPROBE(file_open_entry, struct inode* inode, struct file* file) { return probe_entry(file, 0, 0); }

SEC("kretprobe/vfs_open")

int BPF_KRETPROBE(file_open_exit) { return probe_exit(ctx, F_OPEN, 0); }

SEC("kprobe/vfs_fsync")

int BPF_KPROBE(file_sync_entry, struct file* file, loff_t start, loff_t end) { return probe_entry(file, start, end); }

SEC("kretprobe/vfs_fsync")

int BPF_KRETPROBE(file_sync_exit) { return probe_exit(ctx, F_FSYNC, 0); }

#ifdef DUMMIES
SEC("fentry/dummy_file_read")

int BPF_PROG(file_read_fentry, struct kiocb* iocb)
{
  struct file* fp = iocb->ki_filp;
  loff_t start = iocb->ki_pos;

  return probe_entry(fp, start, 0);
}

SEC("fexit/dummy_file_read")

int BPF_PROG(file_read_fexit, struct kiocb* iocb, struct iov_iter* to, ssize_t ret)
{
  return probe_exit(ctx, F_READ, ret);
}

SEC("fentry/dummy_file_write")

int BPF_PROG(file_write_fentry, struct kiocb* iocb)
{
  struct file* fp = iocb->ki_filp;
  loff_t start = iocb->ki_pos;

  return probe_entry(fp, start, 0);
}

SEC("fexit/dummy_file_write")

int BPF_PROG(file_write_fexit, struct kiocb* iocb, struct iov_iter* from, ssize_t ret)
{
  return probe_exit(ctx, F_WRITE, ret);
}

SEC("fentry/dummy_file_open")

int BPF_PROG(file_open_fentry, struct inode* inode, struct file* file) { return probe_entry(file, 0, 0); }

SEC("fexit/dummy_file_open")

int BPF_PROG(file_open_fexit) { return probe_exit(ctx, F_OPEN, 0); }

SEC("fentry/dummy_file_sync")

int BPF_PROG(file_sync_fentry, struct file* file, loff_t start, loff_t end) { return probe_entry(file, start, end); }

SEC("fexit/dummy_file_sync")

int BPF_PROG(file_sync_fexit) { return probe_exit(ctx, F_FSYNC, 0); }
#endif

char LICENSE[] SEC("license") = "GPL";
