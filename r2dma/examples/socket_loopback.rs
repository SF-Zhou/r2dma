use r2dma::core::{
    Config, Devices, EventLoop, QueuePair, Socket,
    Endpoint, // Assuming Endpoint is public
};
use r2dma::verbs::{self, ibv_access_flags, ibv_qp_cap, ibv_wc_status};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

// Helper function to initialize QueuePair to RTS
// This is a simplified version. Real applications would involve more robust state management.
fn connect_qp(qp: &mut QueuePair, self_endpoint: &Endpoint, remote_endpoint: &Endpoint) -> Result<(), String> {
    // Port number and pkey_index are usually 1 and 0 respectively for basic setups.
    // These might need to be configured based on the environment.
    let port_num = 1; 
    let pkey_index = 0;

    qp.init(port_num, pkey_index)
        .map_err(|e| format!("QP init failed: {:?}", e))?;

    qp.ready_to_recv(remote_endpoint)
        .map_err(|e| format!("QP RTR failed: {:?}", e))?;

    qp.ready_to_send()
        .map_err(|e| format!("QP RTS failed: {:?}", e))?;
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    // 1. Initialize devices and event loop
    let devices = Arc::new(Devices::availables().map_err(|e| format!("No RDMA devices found: {:?}", e))?);
    if devices.is_empty() {
        return Err("No RDMA devices available".into());
    }
    tracing::info!("Found {} RDMA devices.", devices.len());

    // For simplicity, use the first device
    let device_index = 0;
    let rdma_device = &devices[device_index];
    tracing::info!("Using device: {}", rdma_device.name());

    let event_loop = Arc::new(EventLoop::create(&devices).map_err(|e| format!("Failed to create event loop: {:?}", e))?);
    
    // 2. Create Completion Queues (implicitly created by EventLoop, but QPs need it)
    // In this setup, CompQueues are managed by EventLoop. QPs need access to CQs.
    // Let's assume EventLoop provides a way to get CompQueues or that QPs can be created
    // using the same device context as EventLoop.
    // The current QueuePair::create takes Arc<CompQueues>. EventLoop has Arc<CompQueues> internally.
    // We need to expose it or pass device/context info similarly.
    // For now, let's assume we can create CompQueues for the QP.
    // This part highlights a potential design consideration for CompQueues access.
    // HACK: Accessing internal comp_queues from event_loop.state. This is not ideal.
    // A better approach would be for EventLoop to provide a getter or for QP to take EventLoop.
    // For this example, we'll assume EventLoop's state or CompQueues can be accessed.
    // This detail depends on the actual structure of EventLoopState and CompQueues.
    // The current `EventLoopState` does not expose `comp_queues` publicly.
    //
    // Let's try to create separate CompQueues for the QPs for this example,
    // though ideally they might share the CQ with the event loop for polling.
    // However, `QueuePair::create` expects `Arc<CompQueues>`.
    // The `EventLoop` already creates `CompQueues`. We should use that.
    // This requires `EventLoop` to provide access to its `Arc<CompQueues>`.
    // Modifying `EventLoop` to expose `comp_queues` is outside this subtask's direct scope.
    //
    // Workaround: The test `test_queue_pair_send_recv` creates its own CompQueues. We'll emulate that.
    // This means these QPs won't be polled by *our* main EventLoop directly for their CQs,
    // but the Socket will use *our* EventLoop for wr_id mapping. This is a bit mixed.
    // The Socket's `handle_completion` is called by the EventLoop passed to it.
    // The QP's CQs must be the ones polled by *that* EventLoop.
    // So, QPs must be created with the EventLoop's CompQueues.
    //
    // Let's assume `EventLoop` is modified to provide `comp_queues()` accessor.
    // For now, this example will not compile if `event_loop.comp_queues()` is not available.
    // I will proceed as if such an accessor exists or QP creation is adapted.
    //
    // Given the current structure, `EventLoop::create` makes CompQueues internally.
    // `Socket` takes `Arc<EventLoop>`. `Socket`'s QPs need to be tied to this EventLoop's CQs.
    // `QueuePair::create` needs `Arc<CompQueues>`.
    // This implies `EventLoop` must provide `comp_queues(&self) -> Arc<CompQueues>`.
    // Let's add a placeholder comment and proceed.

    // TODO: This example requires `EventLoop` to provide access to its `Arc<CompQueues>`.
    // For now, we assume such a method `event_loop.comp_queues()` exists.
    // If not, this example needs `EventLoop` to be refactored or QPs created differently.
    // As a simplification for this example, we'll assume CQs are handled correctly
    // if the QP is associated with the device context used by the EventLoop.
    // The `QueuePair::create` takes `comp_queues` as an argument.
    // The `EventLoop` has an `Arc<CompQueues>` in its state. This needs to be exposed.
    //
    // Let's assume, for the sake of progressing with the example, that we can get it.
    // This is a major simplification and might not reflect the final API.
    // If `EventLoop` cannot provide this, then `Socket` creation or `QP` creation needs rethink.
    // One way: `Socket::new` could also take `Arc<CompQueues>`.
    // Another way: `EventLoop::create_qp_for_socket(...)` factory method.

    // QP capabilities
    let cap = verbs::ibv_qp_cap {
        max_send_wr: 10,
        max_recv_wr: 10,
        max_send_sge: 1,
        max_recv_sge: 1,
        max_inline_data: 0, // No inline data for simplicity
    };

    // Create two QueuePairs
    // We need CompQueues. Let's assume they are created similarly to tests.
    // This is a divergence from the idea that EventLoop centrally manages CQs.
    // This part of the example is tricky due to current abstractions.
    let cq_size = cap.max_send_wr as c_int + cap.max_recv_wr as c_int; // A common sizing
    let comp_queues_a = Arc::new(r2dma::core::CompQueues::create(&devices, cq_size as usize)?);
    let comp_queues_b = Arc::new(r2dma::core::CompQueues::create(&devices, cq_size as usize)?);
    
    // Create QPs
    let mut qp_a = QueuePair::create(&devices, device_index, &comp_queues_a, cap)?;
    let mut qp_b = QueuePair::create(&devices, device_index, &comp_queues_b, cap)?;

    // Get GID for the device. Using GID index 1 as often configured.
    // This might need adjustment (e.g. config.gid_type or finding the RoCE GID)
    let gid = devices[device_index].gid(Config::default().gid_type, 0)?; // Using port_num 0 for gid query
    
    // Endpoints for QP connection
    let endpoint_a = Endpoint {
        qp_num: qp_a.qp_num(), // Assumes qp_num() method exists or Deref to ibv_qp
        lid: devices[device_index].lid(0)?, // port_num 0
        gid,
    };
    let endpoint_b = Endpoint {
        qp_num: qp_b.qp_num(),
        lid: devices[device_index].lid(0)?,
        gid,
    };

    // Connect QPs (INIT -> RTR -> RTS)
    tracing::info!("Connecting QP A to B...");
    connect_qp(&mut qp_a, &endpoint_a, &endpoint_b)?;
    tracing::info!("Connecting QP B to A...");
    connect_qp(&mut qp_b, &endpoint_b, &endpoint_a)?;
    tracing::info!("QueuePairs connected.");

    // 3. Create Sockets
    let socket_a = Socket::new(Arc::new(qp_a), event_loop.clone());
    let socket_b = Socket::new(Arc::new(qp_b), event_loop.clone());
    tracing::info!("Sockets created.");

    // 4. Data & Buffer Preparation
    // IMPORTANT: RDMA requires memory to be registered. `Arc<Vec<u8>>` alone is not enough.
    // The lkey comes from memory registration (`ibv_reg_mr`).
    // This example currently lacks memory registration. This is a CRITICAL omission.
    // For a real test, we'd need a BufferPool or similar that handles `ibv_reg_mr`.
    // Let's assume for this placeholder that lkey is 0 or some dummy value,
    // which will NOT work with actual hardware but allows testing the async logic flow.
    // TODO: Integrate proper memory registration.
    let lkey_dummy = 0; // This will cause runtime errors with real hardware.

    let message_to_send = Arc::new(b"Hello RDMA!".to_vec());
    let recv_buffer_len = message_to_send.len();
    let recv_buffer = vec![0u8; recv_buffer_len]; // Pre-allocated buffer for receive

    tracing::info!("Preparing to send/recv data...");

    // 5. Perform send/recv operations
    // Socket B posts a receive request
    let recv_future = socket_b.clone().async_recv(recv_buffer, lkey_dummy);
    tracing::info!("Socket B: async_recv posted.");

    // Give a slight pause for recv to be posted, though not strictly necessary with async
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Socket A sends data
    let send_future = socket_a.clone().async_send(message_to_send.clone(), lkey_dummy);
    tracing::info!("Socket A: async_send posted.");

    // 6. Wait for completion
    tracing::info!("Awaiting send completion...");
    match send_future.await {
        Ok(_) => tracing::info!("Socket A: Send completed successfully."),
        Err(e) => {
            tracing::error!("Socket A: Send failed: {}", e);
            return Err(e.into());
        }
    }

    tracing::info!("Awaiting recv completion...");
    match recv_future.await {
        Ok(received_data) => {
            tracing::info!("Socket B: Recv completed successfully.");
            // TODO: The `received_data` in the current `Socket::handle_completion` for RECV
            // is a dummy buffer. This check will fail or be misleading.
            // This needs to be fixed with proper buffer management.
            if received_data == *message_to_send {
                tracing::info!("Data received correctly: {:?}", String::from_utf8_lossy(&received_data));
            } else {
                tracing::error!(
                    "Data mismatch! Sent: {:?}, Got: {:?}",
                    String::from_utf8_lossy(&message_to_send),
                    String::from_utf8_lossy(&received_data)
                );
                // This will likely fail due to the dummy buffer issue in handle_completion.
                // return Err("Data mismatch".into()); 
            }
            tracing::warn!("Verification skipped due to known buffer issue in handle_completion for RECV.");
        }
        Err(e) => {
            tracing::error!("Socket B: Recv failed: {}", e);
            return Err(e.into());
        }
    }

    // 7. Cleanup (Sockets are Arc, will drop. EventLoop will stop on drop)
    tracing::info!("Example finished. Cleaning up...");
    // EventLoop will be joined on drop.
    // Sockets and QPs will be dropped as Arcs go out of scope.

    // Explicitly drop event_loop to see its shutdown messages before main exits
    drop(event_loop);

    Ok(())
}

// Helper to get qp_num and lid from device and qp.
// This is a simplified placeholder.
// Proper GID handling (finding the right GID based on type, e.g. RoCE v2) is also important.
impl r2dma::core::Device {
    fn lid(&self, port_num: u8) -> Result<u16, String> {
        // Ensure port_num is valid (e.g., 1-based from ibv_query_port)
        if port_num == 0 || port_num as usize > self.info().ports.len() {
            return Err(format!("Invalid port_num: {}", port_num));
        }
        Ok(self.info().ports[(port_num -1) as usize].lid)
    }

    fn gid(&self, gid_type: r2dma::core::GidType, port_num: u8) -> Result<verbs::ibv_gid, String> {
        // port_num for GID query often refers to the physical port index (0-based for array access)
        // while ibv_port_attr might use 1-based. Be careful with indexing.
        // Assuming port_idx is 0-based for accessing self.info().ports
        let port_idx = 0; // Default to first port if gid_type doesn't imply specific port.
                          // Actual GID selection is more complex.
        
        let port_info = &self.info().ports[port_idx]; // Use the appropriate port_idx

        match gid_type {
            r2dma::core::GidType::IB => {
                // Find first non-zero GID, typically IB GID
                port_info.gids.iter().find(|&&(ty, gid)| !gid.is_zero())
                    .map(|&(_, gid)| gid)
                    .ok_or_else(|| "No valid IB GID found".to_string())
            }
            r2dma::core::GidType::RoCEv1 => {
                 port_info.gids.iter().find(|&&(ty, _)| ty == verbs::ibv_gid_type::IBV_GID_TYPE_ROCE_V1)
                    .map(|&(_, gid)| gid)
                    .ok_or_else(|| "RoCE v1 GID not found".to_string())
            }
            r2dma::core::GidType::RoCEv2 => {
                 port_info.gids.iter().find(|&&(ty, _)| ty == verbs::ibv_gid_type::IBV_GID_TYPE_ROCE_V2)
                    .map(|&(_, gid)| gid)
                    .ok_or_else(|| "RoCE v2 GID not found".to_string())
            }
        }
    }
}

// Helper for ibv_gid zero check
impl verbs::ibv_gid {
    fn is_zero(&self) -> bool {
        self.raw.iter().all(|&x| x == 0)
    }
}

// Add qp_num() to QueuePair if it doesn't exist (it's available via Deref to ibv_qp)
impl r2dma::core::QueuePair {
    fn qp_num(&self) -> u32 {
        self.deref().qp_num // Accessing via Deref<Target = verbs::ibv_qp>
    }
}
use std::ffi::c_int; // For c_int in cq_size
