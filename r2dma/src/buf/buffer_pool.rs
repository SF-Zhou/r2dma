use crate::*;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

pub struct BufferPool {
    buffer: RegisteredBuffer,
    block_size: usize,
    free_list: Mutex<Vec<usize>>,
}

pub struct Buffer {
    pool: Arc<BufferPool>,
    idx: usize,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.pool.deallocate(self.idx);
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let start = self.idx * self.pool.block_size;
        &self.pool.buffer[start..start + self.pool.block_size]
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let buf: &[u8] = self.deref();
        unsafe { std::slice::from_raw_parts_mut(buf.as_ptr() as *mut u8, buf.len()) }
    }
}

impl Buffer {
    pub fn lkey(&self, device: &Device) -> u32 {
        self.pool.buffer.lkey(device.index())
    }

    pub fn rkey(&self, device: &Device) -> u32 {
        self.pool.buffer.rkey(device.index())
    }
}

impl BufferPool {
    pub fn create(block_size: usize, block_count: usize, devices: &Devices) -> Result<Arc<Self>> {
        let buffer_size = block_size * block_count;
        let buffer = RegisteredBuffer::new(buffer_size, devices)?;
        let free_list = Mutex::new((0..block_count).collect());
        Ok(Arc::new(Self {
            buffer,
            block_size,
            free_list,
        }))
    }

    pub fn allocate(self: &Arc<Self>) -> Result<Buffer> {
        let mut free_list = self.free_list.lock().unwrap();
        match free_list.pop() {
            Some(idx) => Ok(Buffer {
                pool: self.clone(),
                idx,
            }),
            None => Err(Error::AllocMemoryFailed),
        }
    }

    fn deallocate(&self, idx: usize) {
        let mut free_list = self.free_list.lock().unwrap();
        free_list.push(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer() {
        const LEN: usize = 1 << 20;
        let devices = Device::avaiables(&DeviceConfig::default()).unwrap();
        let buffer_pool = BufferPool::create(LEN, 32, &devices).unwrap();
        let mut buffer = buffer_pool.allocate().unwrap();
        assert_eq!(buffer.len(), LEN);
        buffer.fill(1);

        let mut another = buffer_pool.allocate().unwrap();
        assert_ne!(buffer.as_ptr(), another.as_ptr());
        another.fill(2);
        drop(another);
        drop(buffer);

        let buffer = buffer_pool.allocate().unwrap();
        assert_eq!(buffer.len(), LEN);
        buffer.iter().all(|&x| x == 1);

        let another = buffer_pool.allocate().unwrap();
        assert_eq!(another.len(), LEN);
        another.iter().all(|&x| x == 2);
    }

    #[test]
    fn test_send_recv() {
        const LEN: usize = 1 << 20;
        let devices = Device::avaiables(&DeviceConfig::default()).unwrap();
        let buffer_pool = BufferPool::create(LEN, 32, &devices).unwrap();

        let cap = ibv::ibv_qp_cap {
            max_send_wr: 64,
            max_recv_wr: 64,
            max_send_sge: 1,
            max_recv_sge: 1,
            max_inline_data: 0,
        };

        let device = devices.first().unwrap();
        let comp_queue_a = ibv::CompQueue::create(device.context(), 128, None).unwrap();
        let mut queue_pair_a = ibv::QueuePair::create(device.pd(), &comp_queue_a, cap).unwrap();
        let comp_queue_b = ibv::CompQueue::create(device.context(), 128, None).unwrap();
        let mut queue_pair_b = ibv::QueuePair::create(device.pd(), &comp_queue_b, cap).unwrap();

        queue_pair_a.init(1, 0).unwrap();
        queue_pair_b.init(1, 0).unwrap();

        let mut recv_buf = buffer_pool.allocate().unwrap();
        recv_buf.fill(0);
        let mut recv_sge = ibv::ibv_sge {
            addr: recv_buf.as_ptr() as _,
            length: recv_buf.len() as _,
            lkey: recv_buf.lkey(device),
        };
        let mut recv_wr = ibv::ibv_recv_wr {
            wr_id: 1,
            sg_list: &mut recv_sge as *mut _,
            num_sge: 1,
            next: std::ptr::null_mut(),
        };
        assert_eq!(queue_pair_b.post_recv(&mut recv_wr), 0);

        let gid = device.context().query_gid(1, 1).unwrap();
        queue_pair_a
            .ready_to_recv(&ibv::Endpoint {
                qp_num: queue_pair_b.qp_num,
                lid: 0,
                gid,
            })
            .unwrap();
        queue_pair_b
            .ready_to_recv(&ibv::Endpoint {
                qp_num: queue_pair_a.qp_num,
                lid: 0,
                gid,
            })
            .unwrap();

        queue_pair_a.ready_to_send().unwrap();
        queue_pair_b.ready_to_send().unwrap();

        let mut wcs_b = vec![ibv::ibv_wc::default(); 128];
        assert!(comp_queue_b.poll_cq(&mut wcs_b).unwrap().is_empty());

        let mut send_buf = buffer_pool.allocate().unwrap();
        send_buf.fill(1);
        let mut send_sge = ibv::ibv_sge {
            addr: send_buf.as_ptr() as _,
            length: send_buf.len() as _,
            lkey: send_buf.lkey(device),
        };
        let mut send_wr = ibv::ibv_send_wr {
            wr_id: 2,
            sg_list: &mut send_sge as *mut _,
            num_sge: 1,
            opcode: ibv::ibv_wr_opcode::IBV_WR_SEND,
            send_flags: ibv::ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..Default::default()
        };
        assert_eq!(queue_pair_a.post_send(&mut send_wr), 0);

        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut wcs_a = vec![ibv::ibv_wc::default(); 128];
        let comp_a = comp_queue_a.poll_cq(&mut wcs_a).unwrap();
        assert_eq!(comp_a.len(), 1);
        assert_eq!(comp_a[0].wr_id, 2);
        assert_eq!(comp_a[0].qp_num, queue_pair_a.qp_num);
        assert_eq!(comp_a[0].status, ibv::ibv_wc_status::IBV_WC_SUCCESS);

        let comp_b = comp_queue_b.poll_cq(&mut wcs_b).unwrap();
        assert_eq!(comp_b.len(), 1);
        assert_eq!(comp_b[0].wr_id, 1);
        assert_eq!(comp_b[0].qp_num, queue_pair_b.qp_num);
        assert_eq!(comp_b[0].status, ibv::ibv_wc_status::IBV_WC_SUCCESS);
        assert_eq!(comp_b[0].byte_len, send_buf.len() as u32);
        assert_eq!(&recv_buf[..], &send_buf[..]);
    }
}
