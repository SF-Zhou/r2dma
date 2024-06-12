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

    let mut builder = bindgen::Builder::default()
        .clang_args(include_paths.iter().map(|p| format!("-I{:?}", p)))
        .header_contents("header.h", "#include <infiniband/verbs.h>")
        .derive_copy(true)
        .derive_debug(true)
        .derive_default(true)
        .generate_comments(false)
        .prepend_enum_name(false)
        .formatter(bindgen::Formatter::Rustfmt)
        .size_t_is_usize(true)
        .translate_enum_integer_types(true)
        .layout_tests(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .opaque_type("pthread_cond_t")
        .opaque_type("pthread_mutex_t")
        .allowlist_type("ibv_access_flags")
        .allowlist_type("ibv_comp_channel")
        .allowlist_type("ibv_context")
        .allowlist_type("ibv_cq")
        .allowlist_type("ibv_device")
        .allowlist_type("ibv_gid")
        .allowlist_type("ibv_mr")
        .allowlist_type("ibv_pd")
        .allowlist_type("ibv_port_attr")
        .allowlist_type("ibv_qp")
        .allowlist_type("ibv_qp_attr_mask")
        .allowlist_type("ibv_qp_init_attr")
        .allowlist_type("ibv_send_flags")
        .allowlist_type("ibv_wc")
        .allowlist_type("ibv_wc_flags")
        .allowlist_type("ibv_wc_status")
        .allowlist_function("ibv_ack_cq_events")
        .allowlist_function("ibv_alloc_pd")
        .allowlist_function("ibv_close_device")
        .allowlist_function("ibv_create_comp_channel")
        .allowlist_function("ibv_create_cq")
        .allowlist_function("ibv_create_qp")
        .allowlist_function("ibv_dealloc_pd")
        .allowlist_function("ibv_dereg_mr")
        .allowlist_function("ibv_destroy_comp_channel")
        .allowlist_function("ibv_destroy_cq")
        .allowlist_function("ibv_destroy_qp")
        .allowlist_function("ibv_free_device_list")
        .allowlist_function("ibv_get_cq_event")
        .allowlist_function("ibv_get_device_guid")
        .allowlist_function("ibv_get_device_list")
        .allowlist_function("ibv_modify_qp")
        .allowlist_function("ibv_req_notify_cq")
        .allowlist_function("ibv_poll_cq")
        .allowlist_function("ibv_post_recv")
        .allowlist_function("ibv_post_send")
        .allowlist_function("ibv_query_gid")
        .allowlist_function("ibv_query_port")
        .allowlist_function("ibv_open_device")
        .allowlist_function("ibv_reg_mr")
        .bitfield_enum("ibv_access_flags")
        .bitfield_enum("ibv_send_flags")
        .bitfield_enum("ibv_wc_flags")
        .bitfield_enum("ibv_qp_attr_mask");

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

    builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
