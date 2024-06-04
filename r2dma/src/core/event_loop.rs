use ibv::WorkCompletion;
use nix::sys::epoll::EpollEvent;

use crate::*;

use std::sync::mpsc;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Task;

const N: usize = 64;

#[derive(Debug, Clone)]
pub struct EventLoop {
    pub buffer_pool: Arc<BufferPool>,
    pub work_pool: Arc<WorkPool>,
    completions: [WorkCompletion; N],
}

impl EventLoop {
    pub fn new(buffer_pool: &Arc<BufferPool>, work_pool: &Arc<WorkPool>) -> Self {
        Self {
            buffer_pool: buffer_pool.clone(),
            work_pool: work_pool.clone(),
            completions: [(); N].map(|_| WorkCompletion::default()),
        }
    }

    pub fn run(&mut self, channel: Arc<Channel>, receiver: mpsc::Receiver<Task>) {
        let mut events = [(); N].map(|_| EpollEvent::empty());
        while !channel.is_stopping() {
            match channel.poll_events(&mut events) {
                Ok(events) => {
                    for event in events {
                        match event.data() {
                            0 => {
                                // just wake up!
                                channel.on_wake_up();
                            }
                            _ => loop {
                                match channel.poll_socket() {
                                    Ok(None) => break,
                                    Ok(Some(socket)) => self.handle_work_completion(socket),
                                    Err(err) => {
                                        tracing::error!("comp channel poll: {:?}", err);
                                        break;
                                    }
                                }
                            },
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("epoll wait failed: {:?}", err);
                    continue;
                }
            }

            while let Ok(_task) = receiver.try_recv() {
                // TODO(SF): handle user events.
            }
        }
        tracing::info!("event_loop is stopped.");
    }

    fn handle_work_completion(&mut self, socket: &Socket) {
        socket.notify().unwrap();

        loop {
            match socket.poll_completions(&mut self.completions) {
                Ok(wcs) => {
                    if wcs.is_empty() {
                        return;
                    }

                    let mut is_error = false;
                    for wc in wcs {
                        let work = WorkRef::new(&self.work_pool, wc.wr_id as _);

                        let result = match work.ty {
                            WorkType::Send => socket.on_send(wc, work),
                            WorkType::Recv => socket.on_recv(wc, work),
                        };

                        is_error |= result.is_err();
                    }

                    if is_error {
                        let _ = socket.set_to_error();
                    }

                    let need_remove = socket.state.work_complete(wcs.len() as u64);
                    if need_remove {
                        // TODO(SF): remove this socket.
                    }
                }
                Err(err) => tracing::error!("poll comp_queue failed: {:?}", err),
            }
        }
    }
}
