// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

use git::Repository;

use crate::task::Task;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Request {
    Start,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Response {
    Progress { output: String },
}

pub struct GitSyncTask {
    handle: Option<thread::JoinHandle<bool>>,
    request_tx: mpsc::Sender<Request>,
    response_rx: mpsc::Receiver<Response>,
    stop_requested: Arc<AtomicBool>,
    output: String,
    success: Option<bool>,
}

impl GitSyncTask {
    pub fn new(repo: Repository) -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();

        let stop_requested = Arc::new(AtomicBool::new(false));
        let stop_requested_clone = stop_requested.clone();
        let handle =
            thread::spawn(move || run_task(repo, request_rx, response_tx, stop_requested_clone));
        Self {
            handle: Some(handle),
            request_tx,
            response_rx,
            stop_requested,
            output: "".into(),
            success: None,
        }
    }
}

fn run_task(
    repo: Repository,
    request_rx: mpsc::Receiver<Request>,
    response_tx: mpsc::Sender<Response>,
    stop_requested_arc: Arc<AtomicBool>,
) -> bool {
    let send_progress = |msg: &str| {
        let msg = Response::Progress { output: msg.into() };
        log::debug!("Sending message: {:?}", msg);
        response_tx.send(msg).unwrap();
    };
    let stop_requested = || stop_requested_arc.load(Ordering::Relaxed);
    let Ok(request) = request_rx.recv() else {
        return false;
    };
    match request {
        Request::Start => {
            send_progress("Fetching latest changes...");
            if let Err(err) = repo.fetch() {
                send_progress(&format!("Error: {}", err));
                return false;
            }
            if stop_requested() {
                return false;
            }

            send_progress("Updating tracking branches...");
            let branches = match repo.list_branches() {
                Ok(x) => x,
                Err(err) => {
                    send_progress(&format!("Error: {}", err));
                    return false;
                }
            };
            if stop_requested() {
                return false;
            }

            for branch in branches {
                if !branch.can_be_fast_forwarded() {
                    continue;
                }
                send_progress(&format!("- {}", branch.name));
                if let Err(err) = repo.fast_forward_branch(&branch) {
                    send_progress(&format!("Error: {}", err));
                    return false;
                }
                if stop_requested() {
                    return false;
                }
                send_progress("All done");
            }
        }
    }
    true
}

impl Task for GitSyncTask {
    fn start(&mut self) {
        self.request_tx.send(Request::Start).unwrap();
    }

    fn update(&mut self) {
        while let Ok(response) = self.response_rx.try_recv() {
            match response {
                Response::Progress { output } => {
                    log::debug!("Received progress. output={output}");
                    self.output.push_str(&format!("{output}\n"));
                }
            }
        }
        if let Some(handle) = &self.handle {
            if handle.is_finished() {
                log::debug!("task finished");
                let handle = self.handle.take().unwrap();
                self.success = Some(handle.join().unwrap());
            }
        }
    }

    fn output(&self) -> &str {
        &self.output
    }

    fn success(&self) -> Option<bool> {
        self.success
    }

    fn title(&self) -> &str {
        "Synchronizing"
    }
}

impl Drop for GitSyncTask {
    fn drop(&mut self) {
        if self.handle.is_some() {
            log::debug!("Sending Stop request");
            self.stop_requested.store(true, Ordering::Relaxed);
            self.update();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{fs, time::Duration};

    use git::{Repository, INITIAL_BRANCH};

    use crate::gitsynctask::GitSyncTask;

    use super::*;

    fn create_empty_commit(repo: &Repository) {
        repo.git("commit", &["--allow-empty", "-m", "Empty"])
            .unwrap();
    }

    #[test]
    fn can_synchronize() {
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let remote_path = tmp_dir.join("remote");
        let local_path = tmp_dir.join("local");

        // Creating remote repo
        fs::create_dir(&remote_path).unwrap();
        let remote_repo = Repository::new(&remote_path);
        remote_repo.init_bare().unwrap();

        // Creating local repo
        fs::create_dir(&local_path).unwrap();
        let remote_url = format!("file://{}", remote_path.display());
        let local_repo = Repository::clone_repository(&local_path, &remote_url).unwrap();

        // Creating commits in main branch
        create_empty_commit(&local_repo);
        create_empty_commit(&local_repo);
        local_repo.push().unwrap();

        // Create branches that can be ff
        local_repo.checkout(INITIAL_BRANCH).unwrap();
        let mut branch_and_target_sha1s: Vec<(String, String)> = vec![];
        for x in 0..=4 {
            let name = format!("can-be-ff-{x}");
            local_repo.create_branch(&name).unwrap();
            create_empty_commit(&local_repo);
            create_empty_commit(&local_repo);
            create_empty_commit(&local_repo);
            let target_sha1 = local_repo.get_current_sha1().unwrap();
            local_repo.push().unwrap();
            local_repo.git("reset", &["--hard", "HEAD~2"]).unwrap();

            branch_and_target_sha1s.push((name, target_sha1));
        }

        let mut task = GitSyncTask::new(local_repo.clone());
        task.start();

        while task.success().is_none() {
            task.update();
            thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(task.success(), Some(true));

        for (name, target_sha1) in branch_and_target_sha1s {
            local_repo.checkout(&name).unwrap();
            let sha1 = local_repo.get_current_sha1().unwrap();
            assert_eq!(sha1, target_sha1);
        }
    }
}
