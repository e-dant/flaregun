fn main() {
    build_bpf::guess_targets().for_each(|target| {
        target.must_build();
        let manifestdir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let outdir = std::env::var("OUT_DIR").unwrap();
        let progname = target.bpf_prog_name();
        let visible = format!("{manifestdir}/src/skel_{progname}.rs");
        let artifact = format!("{outdir}/skel_{progname}.rs");
        std::fs::copy(&artifact, &visible).unwrap();
    });
}
