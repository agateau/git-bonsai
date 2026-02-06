// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

pub struct WorkerController<Progress> {
    stop_requested: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<Progress>,
}

impl<Progress> WorkerController<Progress> {
    fn new(progress_tx: mpsc::Sender<Progress>, stop_requested: Arc<AtomicBool>) -> Self {
        Self {
            stop_requested,
            progress_tx,
        }
    }

    pub fn send_progress(&mut self, progress: Progress) {
        self.progress_tx.send(progress).unwrap();
    }

    pub fn stop_requested(&self) -> bool {
        self.stop_requested.load(Ordering::Relaxed)
    }
}

pub struct Worker<Progress, Output>
where
    Progress: Send + 'static,
    Output: Send + 'static,
{
    handle: Option<thread::JoinHandle<Output>>,
    progress_tx: Option<mpsc::Sender<Progress>>,
    progress_rx: mpsc::Receiver<Progress>,
    progress_messages: VecDeque<Progress>,
    stop_requested: Arc<AtomicBool>,
    response: Option<Output>,
}

impl<Progress, Output> Default for Worker<Progress, Output>
where
    Progress: Send + 'static,
    Output: Send + 'static,
{
    fn default() -> Self {
        let (progress_tx, progress_rx) = mpsc::channel();
        let stop_requested = Arc::new(AtomicBool::new(false));
        Self {
            handle: None,
            progress_tx: Some(progress_tx),
            progress_rx,
            progress_messages: VecDeque::new(),
            stop_requested,
            response: None,
        }
    }
}

impl<Progress, Output> Worker<Progress, Output>
where
    Progress: Send + 'static,
    Output: Send + 'static,
{
    pub fn start<F, Input>(&mut self, worker_fn: F, input: Input)
    where
        Input: Send + 'static,
        F: Fn(Input, WorkerController<Progress>) -> Output + Send + 'static,
    {
        let progress_tx = self.progress_tx.take().unwrap();
        let stop_requested = self.stop_requested.clone();
        let handle = thread::spawn(move || {
            let worker_controller = WorkerController::new(progress_tx, stop_requested);
            worker_fn(input, worker_controller)
        });
        self.handle = Some(handle);
    }

    pub fn pop_progress(&mut self) -> Option<Progress> {
        self.progress_messages.pop_back()
    }

    pub fn update(&mut self) -> &Option<Output> {
        let Some(handle) = &self.handle else {
            assert!(self.response.is_some());
            return &self.response;
        };
        while let Ok(progress) = self.progress_rx.try_recv() {
            log::debug!("got some progress");
            self.progress_messages.push_front(progress);
        }
        if handle.is_finished() {
            log::debug!("task finished");
            let handle = self.handle.take().unwrap();
            self.response = Some(handle.join().unwrap());
        }
        &self.response
    }
}

impl<Progress, Output> Drop for Worker<Progress, Output>
where
    Progress: Send + 'static,
    Output: Send + 'static,
{
    fn drop(&mut self) {
        if self.handle.is_some() {
            log::debug!("Sending Stop request");
            self.stop_requested.store(true, Ordering::Relaxed);
            self.update();
        }
    }
}
