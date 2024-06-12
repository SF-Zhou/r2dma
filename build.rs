use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

fn main() {
    let lib = pkg_config::Config::new()
        .statik(false)
        .probe("libibverbs")
        .unwrap_or_else(|_| panic!("please install libibverbs-dev"));

    let mut include_paths = lib.include_paths.into_iter().collect::<HashSet<_>>();
    include_paths.insert(PathBuf::from("/usr/include"));

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let gen_path = out_path.join("verbs_inline.c");
    let obj_path = out_path.join("verbs_inline.o");
    let lib_path = out_path.join("libverbs_inline.a");

    // 1. generate rust bindings.
    let mut builder = bindgen::Builder::default()
        .clang_args(include_paths.iter().map(|p| format!("-I{:?}", p)))
        .header_contents("header.h", "#include <infiniband/verbs.h>")
        .derive_copy(true)
        .derive_debug(true)
        .derive_default(true)
        .generate_comments(false)
        .generate_inline_functions(true)
        .wrap_static_fns(true)
        .wrap_static_fns_path(&gen_path)
        .prepend_enum_name(false)
        .formatter(bindgen::Formatter::Rustfmt)
        .size_t_is_usize(true)
        .translate_enum_integer_types(true)
        .layout_tests(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .opaque_type("pthread_.*")
        .blocklist_type("timespec")
        .allowlist_function("ibv_.*")
        .allowlist_type("ibv_.*")
        .allowlist_type("ib_uverbs_access_flags")
        .bitfield_enum("ib_uverbs_access_flags")
        .bitfield_enum("ibv_.*_bits")
        .bitfield_enum("ibv_.*_caps")
        .bitfield_enum("ibv_.*_flags")
        .bitfield_enum("ibv_.*_mask")
        .bitfield_enum("ibv_pci_atomic_op_size")
        .bitfield_enum("ibv_port_cap_flags2")
        .bitfield_enum("ibv_rx_hash_fields");

    for name in [
        "ibv_srq",
        "ibv_wq",
        "ibv_qp",
        "ibv_cq",
        "ibv_cq_ex",
        "ibv_context",
    ] {
        builder = builder.no_copy(name).no_debug(name)
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    // 2. compile verbs_inline.
    let clang_output = std::process::Command::new("clang")
        .arg("-O2")
        .arg("-c")
        .arg("-o")
        .arg(&obj_path)
        .arg(&gen_path)
        .args(["-include", "infiniband/verbs.h"])
        .output()
        .unwrap();
    if !clang_output.status.success() {
        panic!(
            "Could not compile object file: {}",
            String::from_utf8_lossy(&clang_output.stderr)
        );
    }

    // 3. archive verbs_inline.
    let lib_output = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&lib_path)
        .arg(&obj_path)
        .output()
        .unwrap();
    if !lib_output.status.success() {
        panic!(
            "Could not emit library file: {}",
            String::from_utf8_lossy(&lib_output.stderr)
        );
    }

    println!("cargo:rustc-link-lib=static=verbs_inline");
    println!("cargo:rustc-link-search=native={}", out_path.display());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
