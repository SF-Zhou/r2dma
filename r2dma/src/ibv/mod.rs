mod verbs;
pub use verbs::*;

mod devices;
pub use devices::Device;

mod gid;
pub use gid::GidType;

mod context;
pub use context::Context;

mod protection_domain;
pub use protection_domain::ProtectionDomain;

mod comp_channel;
pub use comp_channel::CompChannel;

mod comp_queue;
pub use comp_queue::CompQueue;

mod memory_region;
pub use memory_region::MemoryRegion;

mod work_completion;

mod queue_pair;
pub use queue_pair::QueuePair;

pub const ACCESS_FLAGS: u32 = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0;
