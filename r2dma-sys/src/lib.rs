#![allow(deref_nullptr)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use libc::{pthread_cond_t, pthread_mutex_t, timespec};
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
