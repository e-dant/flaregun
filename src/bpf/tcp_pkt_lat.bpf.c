// SPDX-License-Identifier: GPL-2.0
// Copyright (c) 2023 Wenbo Zhang
#include "constants.h"
#include "core_fixes.bpf.h"
#include "vmlinux.h"
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

#define MAX_ENTRIES 10240
#define AF_INET 2
#define MAX_EVENT_SIZE 10240
#define RINGBUF_SIZE (1024 * 256)

volatile const pid_t targ_pid = 0;
volatile const pid_t targ_tgid = 0;
volatile const __u16 targ_sport = 0;
volatile const __u16 targ_dport = 0;
volatile const __u64 min_lat_us = 0;

struct event {
  __u32 saddr[4];
  __u32 daddr[4];
  __u64 lat_us;
  pid_t pid;
  pid_t tid;
  __u16 dport;
  __u16 sport;
  __u16 family;
  u8 task[FL_TASK_COMM_LEN];
} _event = {};

struct {
  __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
  __uint(max_entries, 1);
  __uint(key_size, sizeof(__u32));
  __uint(value_size, MAX_EVENT_SIZE);
} heap SEC(".maps");

struct {
  __uint(type, BPF_MAP_TYPE_RINGBUF);
  __uint(max_entries, RINGBUF_SIZE);
} events SEC(".maps");

static __always_inline void* reserve_buf(__u64 size)
{
  return bpf_ringbuf_reserve(&events, size, 0);
}

static __always_inline long submit_buf(void* ctx, void* buf, __u64 size)
{
   bpf_ringbuf_submit(buf, 0);
   return 0;
}

struct {
  __uint(type, BPF_MAP_TYPE_HASH);
  __uint(max_entries, MAX_ENTRIES);
  __type(key, u64);
  __type(value, u64);
} start SEC(".maps");

static int handle_tcp_probe(struct sock* sk, struct sk_buff* skb)
{
  const struct inet_sock* inet = (struct inet_sock*)(sk);
  u64 sock_ident, ts, len, doff;
  const struct tcphdr* th;

  if (targ_sport && targ_sport != BPF_CORE_READ(inet, inet_sport))
    return 0;
  if (targ_dport && targ_dport != BPF_CORE_READ(sk, __sk_common.skc_dport))
    return 0;
  th = (const struct tcphdr*)BPF_CORE_READ(skb, data);
  doff = BPF_CORE_READ_BITFIELD_PROBED(th, doff);
  len = BPF_CORE_READ(skb, len);
  /* `doff * 4` means `__tcp_hdrlen` */
  if (len <= doff * 4)
    return 0;
  sock_ident = get_sock_ident(sk);
  ts = bpf_ktime_get_ns();
  bpf_map_update_elem(&start, &sock_ident, &ts, 0);
  return 0;
}

static int handle_tcp_rcv_space_adjust(void* ctx, struct sock* sk)
{
  const struct inet_sock* inet = (struct inet_sock*)(sk);
  u64 sock_ident = get_sock_ident(sk);
  u64 id = bpf_get_current_pid_tgid(), *tsp;
  u32 pid = id >> 32, tid = id;
  struct event* eventp;
  s64 lat_us;
  u16 family;

  tsp = bpf_map_lookup_elem(&start, &sock_ident);
  if (! tsp)
    return 0;

  if (targ_pid && targ_pid != pid)
    goto cleanup;
  if (targ_tgid && targ_tgid != tid)
    goto cleanup;

  lat_us = (bpf_ktime_get_ns() - *tsp) / 1000;
  if (lat_us < 0 || lat_us <= min_lat_us)
    goto cleanup;

  eventp = reserve_buf(sizeof(*eventp));
  if (! eventp)
    goto cleanup;

  eventp->pid = pid;
  eventp->tid = tid;
  eventp->lat_us = lat_us;
  eventp->sport = BPF_CORE_READ(inet, inet_sport);
  eventp->dport = BPF_CORE_READ(sk, __sk_common.skc_dport);
  __builtin_memset(eventp->task, 0, sizeof(eventp->task));
  bpf_get_current_comm(&eventp->task, FL_TASK_COMM_LEN);
  family = BPF_CORE_READ(sk, __sk_common.skc_family);
  if (family == AF_INET) {
    eventp->saddr[0] = BPF_CORE_READ(sk, __sk_common.skc_rcv_saddr);
    eventp->daddr[0] = BPF_CORE_READ(sk, __sk_common.skc_daddr);
  } else { /* family == AF_INET6 */
    BPF_CORE_READ_INTO(eventp->saddr, sk, __sk_common.skc_v6_rcv_saddr.in6_u.u6_addr32);
    BPF_CORE_READ_INTO(eventp->daddr, sk, __sk_common.skc_v6_daddr.in6_u.u6_addr32);
  }
  eventp->family = family;
  submit_buf(ctx, eventp, sizeof(*eventp));

cleanup:
  bpf_map_delete_elem(&start, &sock_ident);
  return 0;
}

static int handle_tcp_destroy_sock(void* ctx, struct sock* sk)
{
  u64 sock_ident = get_sock_ident(sk);

  bpf_map_delete_elem(&start, &sock_ident);
  return 0;
}

SEC("tp_btf/tcp_probe")

int BPF_PROG(tcp_probe_btf, struct sock* sk, struct sk_buff* skb) { return handle_tcp_probe(sk, skb); }

SEC("tp_btf/tcp_rcv_space_adjust")

int BPF_PROG(tcp_rcv_space_adjust_btf, struct sock* sk) { return handle_tcp_rcv_space_adjust(ctx, sk); }

SEC("tp_btf/tcp_destroy_sock")

int BPF_PROG(tcp_destroy_sock_btf, struct sock* sk) { return handle_tcp_destroy_sock(ctx, sk); }

SEC("raw_tp/tcp_probe")

int BPF_PROG(tcp_probe, struct sock* sk, struct sk_buff* skb) { return handle_tcp_probe(sk, skb); }

SEC("raw_tp/tcp_rcv_space_adjust")

int BPF_PROG(tcp_rcv_space_adjust, struct sock* sk) { return handle_tcp_rcv_space_adjust(ctx, sk); }

SEC("raw_tp/tcp_destroy_sock")

int BPF_PROG(tcp_destroy_sock, struct sock* sk) { return handle_tcp_destroy_sock(ctx, sk); }

char LICENSE[] SEC("license") = "GPL";
