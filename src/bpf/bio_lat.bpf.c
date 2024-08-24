// SPDX-License-Identifier: GPL-2.0
// Copyright (c) 2020 Wenbo Zhang
#include "vmlinux.h"
#include "constants.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_tracing.h>


#include "core_fixes.bpf.h"

#define MAX_ENTRIES	10240
#define DISK_NAME_LEN 32
#define RWBS_LEN  8

#define MINORBITS 20
#define MINORMASK ((1U << MINORBITS) - 1)

#define MKDEV(ma, mi) (((ma) << MINORBITS) | (mi))

const volatile bool targ_filter_cgroup = false;
const volatile bool targ_filter_queued = false;
const volatile bool targ_filter_dev = false;
const volatile __u32 targ_dev = 0;
const volatile pid_t targ_pid = 0;
const volatile pid_t targ_tgid = 0;
const volatile __u64 min_lat_us = 0;

struct event {
  u8 task[FL_TASK_COMM_LEN];
  __u64 lat_us;
  __u64 q_lat_us;
  __u64 ts;
  __u64 sector;
  __u32 len;
  __u32 pid;
  __u32 cmd_flags;
  __u32 dev;
} _event = {};

extern __u32 LINUX_KERNEL_VERSION __kconfig;

struct {
	__uint(type, BPF_MAP_TYPE_CGROUP_ARRAY);
	__type(key, u32);
	__type(value, u32);
	__uint(max_entries, 1);
} cgroup_map SEC(".maps");

struct piddata {
	u32 pid;
  u32 tgid;
};

struct stage {
	u64 insert;
	u64 issue;
	__u32 dev;
};

struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, MAX_ENTRIES);
	__type(key, struct request *);
	__type(value, struct stage);
} start SEC(".maps");

struct {
	__uint(type, BPF_MAP_TYPE_PERF_EVENT_ARRAY);
	__uint(key_size, sizeof(u32));
	__uint(value_size, sizeof(u32));
} events SEC(".maps");

static __always_inline
int current_piddata(struct request *rq, struct piddata *piddata)
{
  u64 pid_tgid = bpf_get_current_pid_tgid();
  u32 tgid = (u32)pid_tgid;
  piddata->pid = pid_tgid >> 32;
  piddata->tgid = tgid;
  if (targ_pid && piddata->pid != targ_pid)
    return 1;
  if (targ_tgid && tgid != targ_tgid)
    return 1;
  return 0;
}

static __always_inline
int trace_rq_start(struct request *rq, bool insert)
{
	struct stage *stagep, stage = {};
	u64 ts = bpf_ktime_get_ns();

	stagep = bpf_map_lookup_elem(&start, &rq);
	if (!stagep) {
		struct gendisk *disk = get_disk(rq);

		stage.dev = disk ? MKDEV(BPF_CORE_READ(disk, major),
				BPF_CORE_READ(disk, first_minor)) : 0;
		if (targ_filter_dev && targ_dev != stage.dev)
			return 0;
		stagep = &stage;
	}
	if (insert)
		stagep->insert = ts;
	else
		stagep->issue = ts;
	if (stagep == &stage) {
		bpf_map_update_elem(&start, &rq, stagep, 0);
  }
	return 0;
}

SEC("tp_btf/block_rq_insert")
int BPF_PROG(block_rq_insert)
{
	if (targ_filter_cgroup && !bpf_current_task_under_cgroup(&cgroup_map, 0))
		return 0;

	/**
	 * commit a54895fa (v5.11-rc1) changed tracepoint argument list
	 * from TP_PROTO(struct request_queue *q, struct request *rq)
	 * to TP_PROTO(struct request *rq)
	 */
	if (LINUX_KERNEL_VERSION >= KERNEL_VERSION(5, 11, 0))
		return trace_rq_start((void *)ctx[0], true);
	else
		return trace_rq_start((void *)ctx[1], true);
}

SEC("tp_btf/block_rq_issue")
int BPF_PROG(block_rq_issue)
{
	if (targ_filter_cgroup && !bpf_current_task_under_cgroup(&cgroup_map, 0))
		return 0;

	/**
	 * commit a54895fa (v5.11-rc1) changed tracepoint argument list
	 * from TP_PROTO(struct request_queue *q, struct request *rq)
	 * to TP_PROTO(struct request *rq)
	 */
	if (LINUX_KERNEL_VERSION >= KERNEL_VERSION(5, 11, 0))
		return trace_rq_start((void *)ctx[0], false);
	else
		return trace_rq_start((void *)ctx[1], false);
}

SEC("tp_btf/block_rq_complete")
int BPF_PROG(block_rq_complete, struct request *rq, int error,
	     unsigned int nr_bytes)
{
	if (targ_filter_cgroup && !bpf_current_task_under_cgroup(&cgroup_map, 0))
		return 0;

	u64 ts = bpf_ktime_get_ns();
	struct event event = {};
	struct stage *stagep;
	s64 delta;

	stagep = bpf_map_lookup_elem(&start, &rq);
	if (!stagep)
		return 0;
	delta = (s64)(ts - stagep->issue);
  u64 delta_us = delta / 1000ul;
	if (delta < 0 || delta_us < min_lat_us)
		goto cleanup;
  struct piddata piddata = {};
  if (current_piddata(rq, &piddata))
    goto cleanup;
  bpf_get_current_comm(&event.task, sizeof(event.task));
  event.pid = piddata.pid;
	event.lat_us = delta_us;
	if (targ_filter_queued && BPF_CORE_READ(rq, q, elevator)) {
		if (!stagep->insert)
			event.q_lat_us = -1; /* missed or don't insert entry */
		else
			event.q_lat_us = (stagep->issue - stagep->insert) / 1000;
	}
	event.ts = ts;
	event.sector = BPF_CORE_READ(rq, __sector);
	event.len = BPF_CORE_READ(rq, __data_len);
	event.cmd_flags = BPF_CORE_READ(rq, cmd_flags);
	event.dev = stagep->dev;
	bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event,
			sizeof(event));

cleanup:
	bpf_map_delete_elem(&start, &rq);
	return 0;
}

char LICENSE[] SEC("license") = "GPL";
