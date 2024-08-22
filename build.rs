fn main() {
    build_bpf::guess_targets().for_each(|target| {
        target.must_build();
    });
}
