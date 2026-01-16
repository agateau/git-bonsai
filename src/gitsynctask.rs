// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use git::{Branch, Repository};

use crate::{
    task::Task,
    worker::{Worker, WorkerController},
};

pub struct GitSyncTask {
    repo: Repository,
    worker: Worker<String, bool>,
    output: String,
    success: Option<bool>,
}

impl GitSyncTask {
    pub fn new(repo: Repository) -> Self {
        Self {
            repo,
            worker: Worker::<String, bool>::default(),
            output: "".into(),
            success: None,
        }
    }
}

impl Task for GitSyncTask {
    fn start(&mut self) {
        self.worker.start(&process, self.repo.clone());
    }

    fn update(&mut self) {
        if let Some(x) = self.worker.update() {
            self.success = Some(*x);
        }
        while let Some(progress) = self.worker.pop_progress() {
            self.output.push_str(&progress);
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

fn process(repo: Repository, mut controller: WorkerController<String>) -> bool {
    log::debug!("Starting process");
    controller.send_progress("Fetching changes... ".into());
    if let Err(err) = repo.fetch() {
        controller.send_progress(format!("ERROR: {}\n", err));
        return false;
    }
    controller.send_progress("OK\n".into());
    if controller.stop_requested() {
        return false;
    }

    let branches = match repo.list_branches() {
        Ok(x) => x,
        Err(err) => {
            controller.send_progress(format!("ERROR: {}\n", err));
            return false;
        }
    };
    if controller.stop_requested() {
        return false;
    }

    let mut success = true;
    let branches: Vec<Branch> = branches
        .into_iter()
        .filter(Branch::can_be_fast_forwarded)
        .collect();

    for branch in branches {
        controller.send_progress(format!("Updating {}... ", branch.name));
        match repo.fast_forward_branch(&branch) {
            Ok(()) => {
                controller.send_progress("OK\n".into());
            }
            Err(err) => {
                controller.send_progress(format!("ERROR: {}\n", err));
                success = false;
            }
        }
        if controller.stop_requested() {
            return false;
        }
    }
    controller.send_progress("Finished\n".into());
    success
}

#[cfg(test)]
mod test {
    use std::{fs, thread, time::Duration};

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
