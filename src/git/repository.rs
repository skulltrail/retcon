#![allow(clippy::missing_errors_doc)]

use crate::error::{HistError, Result};
use crate::git::commit::{CommitData, CommitId};
use git2::{Repository as Git2Repository, RepositoryState, StatusOptions};
use std::path::Path;

/// Wrapper around `git2::Repository` with convenience methods for retcon
pub struct Repository {
    inner: Git2Repository,
}

impl Repository {
    /// Open a repository at the given path
    ///
    /// # Errors
    /// Returns an error if the path is not a git repository or the repository is in an invalid state.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let inner = Git2Repository::discover(path)
            .map_err(|_| HistError::NotARepository(path.display().to_string()))?;

        let repo = Self { inner };
        repo.validate_state()?;
        Ok(repo)
    }

    /// Open a repository at the current directory
    pub fn open_current_dir() -> Result<Self> {
        Self::open(".")
    }

    /// Validate that the repository is in a clean state for history editing
    fn validate_state(&self) -> Result<()> {
        // Check repository state - only block on active operations
        match self.inner.state() {
            RepositoryState::Clean => {}
            RepositoryState::Rebase
            | RepositoryState::RebaseInteractive
            | RepositoryState::RebaseMerge => {
                return Err(HistError::RebaseInProgress);
            }
            RepositoryState::Merge => {
                return Err(HistError::MergeInProgress);
            }
            _ => {
                return Err(HistError::RewriteFailed(
                    "Repository is in an unsupported state".to_string(),
                ));
            }
        }

        // Note: Uncommitted changes are allowed for browsing.
        // The check is performed before applying changes in rewrite_history().

        Ok(())
    }

    /// Validate that the working tree is clean before rewriting history
    pub fn validate_clean_for_rewrite(&self) -> Result<()> {
        if self.has_uncommitted_changes()? {
            return Err(HistError::DirtyWorkingTree);
        }
        Ok(())
    }

    /// Check if there are any uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(false)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = self.inner.statuses(Some(&mut opts))?;
        Ok(!statuses.is_empty())
    }

    /// Get the current branch name
    pub fn current_branch_name(&self) -> Result<String> {
        let head = self.inner.head()?;
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    }

    /// Check if the current branch has an upstream
    pub fn has_upstream(&self) -> Result<bool> {
        let head = self.inner.head()?;
        if !head.is_branch() {
            return Ok(false);
        }

        let branch_name = head.shorthand().unwrap_or("");
        let branch = self
            .inner
            .find_branch(branch_name, git2::BranchType::Local)?;
        Ok(branch.upstream().is_ok())
    }

    /// Load commits from HEAD, up to the specified limit
    pub fn load_commits(&self, limit: usize) -> Result<Vec<CommitData>> {
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for (count, oid_result) in revwalk.enumerate() {
            if count >= limit {
                break;
            }

            let oid = oid_result?;
            let commit = self.inner.find_commit(oid)?;
            commits.push(CommitData::from_git2_commit(&commit));
        }

        if commits.is_empty() {
            return Err(HistError::NoCommits);
        }

        Ok(commits)
    }

    /// Load commits in a specific range (exclusive start, inclusive end)
    #[allow(dead_code)]
    pub fn load_commits_range(
        &self,
        from: Option<CommitId>,
        to: CommitId,
        limit: usize,
    ) -> Result<Vec<CommitData>> {
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push(to.0)?;

        if let Some(from_id) = from {
            revwalk.hide(from_id.0)?;
        }

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for (count, oid_result) in revwalk.enumerate() {
            if count >= limit {
                break;
            }

            let oid = oid_result?;
            let commit = self.inner.find_commit(oid)?;
            commits.push(CommitData::from_git2_commit(&commit));
        }

        Ok(commits)
    }

    /// Get the total number of commits in the repository
    #[allow(dead_code)]
    pub fn commit_count(&self) -> Result<usize> {
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push_head()?;
        Ok(revwalk.count())
    }

    /// Find a commit by its ID
    #[allow(dead_code)]
    pub fn find_commit(&self, id: CommitId) -> Result<CommitData> {
        let commit = self.inner.find_commit(id.0)?;
        Ok(CommitData::from_git2_commit(&commit))
    }

    /// Get the inner git2 repository (for rewriting operations)
    #[must_use]
    pub fn inner(&self) -> &Git2Repository {
        &self.inner
    }

    /// Get mutable reference to inner git2 repository
    #[allow(dead_code)]
    pub fn inner_mut(&mut self) -> &mut Git2Repository {
        &mut self.inner
    }

    /// Create a backup reference before rewriting
    pub fn create_backup_ref(&self, branch_name: &str) -> Result<()> {
        let head = self.inner.head()?;
        let commit = head.peel_to_commit()?;

        let backup_ref = format!("refs/original/heads/{branch_name}");
        self.inner
            .reference(
                &backup_ref,
                commit.id(),
                false, // Don't overwrite if exists
                "retcon: backup before rewrite",
            )
            .ok(); // Ignore error if already exists

        Ok(())
    }

    /// Get the HEAD commit ID
    #[allow(dead_code)]
    pub fn head_commit_id(&self) -> Result<CommitId> {
        let head = self.inner.head()?;
        let commit = head.peel_to_commit()?;
        Ok(CommitId(commit.id()))
    }

    /// Stash uncommitted changes if any exist
    ///
    /// Returns true if changes were stashed, false if working tree was clean.
    /// The stash is created with a special message to identify it as auto-created.
    pub fn stash_changes(&mut self) -> Result<bool> {
        if !self.has_uncommitted_changes()? {
            return Ok(false);
        }

        // Get signature for stash
        let signature = self.inner.signature()?;

        // Create stash with a recognizable message
        self.inner.stash_save(
            &signature,
            "retcon: auto-stash before history rewrite",
            Some(git2::StashFlags::INCLUDE_UNTRACKED),
        )?;

        Ok(true)
    }

    /// Restore previously stashed changes
    ///
    /// This pops the most recent stash entry. Should only be called after
    /// `stash_changes` returned true.
    pub fn unstash_changes(&mut self) -> Result<()> {
        self.inner.stash_pop(0, None)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;

    /// Helper to create a test git repository with some commits
    fn create_test_repo() -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize repository with explicit "main" branch name
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Git2Repository::init_opts(&repo_path, &opts).unwrap();

        // Explicitly set HEAD to ensure "main" branch regardless of system git config
        repo.set_head("refs/heads/main").unwrap();

        // Configure user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        // Create initial commit
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            // Create a file
            let file_path = repo_path.join("test.txt");
            fs::write(&file_path, "test content").unwrap();
            index.add_path(std::path::Path::new("test.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Create second commit
        let tree_id = {
            let mut index = repo.index().unwrap();
            let file_path = repo_path.join("test2.txt");
            fs::write(&file_path, "test content 2").unwrap();
            index.add_path(std::path::Path::new("test2.txt")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Second commit", &tree, &[&parent])
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let result = Repository::open("/nonexistent/path");
        assert!(matches!(result, Err(HistError::NotARepository(_))));
    }

    #[test]
    #[serial]
    fn test_open_valid_repo() {
        let (_temp_dir, repo_path) = create_test_repo();
        let result = Repository::open(&repo_path);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_load_commits() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let commits = repo.load_commits(10).unwrap();
        assert_eq!(commits.len(), 2);

        // Most recent commit first
        assert_eq!(commits[0].summary, "Second commit");
        assert_eq!(commits[1].summary, "Initial commit");
    }

    #[test]
    #[serial]
    fn test_load_commits_with_limit() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let commits = repo.load_commits(1).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "Second commit");
    }

    #[test]
    #[serial]
    fn test_current_branch_name() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let branch_name = repo.current_branch_name().unwrap();
        assert_eq!(branch_name, "main");
    }

    #[test]
    #[serial]
    fn test_has_upstream_false() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        // New repo has no upstream
        let has_upstream = repo.has_upstream().unwrap();
        assert!(!has_upstream);
    }

    #[test]
    #[serial]
    fn test_has_uncommitted_changes_clean() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let has_changes = repo.has_uncommitted_changes().unwrap();
        assert!(!has_changes);
    }

    #[test]
    #[serial]
    fn test_has_uncommitted_changes_dirty() {
        let (_temp_dir, repo_path) = create_test_repo();

        // Create a modified file
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "modified content").unwrap();

        // Open the underlying git2 repo directly (not our wrapper which validates state)
        let git_repo = Git2Repository::open(&repo_path).unwrap();
        let mut opts = StatusOptions::new();
        opts.include_untracked(false)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = git_repo.statuses(Some(&mut opts)).unwrap();
        assert!(!statuses.is_empty());
    }

    #[test]
    #[serial]
    fn test_commit_count() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let count = repo.commit_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    #[serial]
    fn test_find_commit() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let commits = repo.load_commits(10).unwrap();
        let first_id = commits[0].id;

        let found = repo.find_commit(first_id).unwrap();
        assert_eq!(found.id, first_id);
        assert_eq!(found.summary, "Second commit");
    }

    #[test]
    #[serial]
    fn test_head_commit_id() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let head_id = repo.head_commit_id().unwrap();
        let commits = repo.load_commits(1).unwrap();

        assert_eq!(head_id, commits[0].id);
    }

    #[test]
    #[serial]
    fn test_create_backup_ref() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        repo.create_backup_ref("main").unwrap();

        // Verify backup ref was created
        let git_repo = repo.inner();
        let backup_ref = git_repo.find_reference("refs/original/heads/main");
        assert!(backup_ref.is_ok());
    }

    #[test]
    #[serial]
    fn test_load_commits_range() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let all_commits = repo.load_commits(10).unwrap();
        let head_id = all_commits[0].id;

        // Load commits excluding the first one
        let commits = repo.load_commits_range(Some(head_id), head_id, 10).unwrap();
        assert_eq!(commits.len(), 0); // Exclusive range, so no commits
    }

    #[test]
    #[serial]
    fn test_commit_data_from_git2() {
        let (_temp_dir, repo_path) = create_test_repo();
        let git_repo = Git2Repository::open(&repo_path).unwrap();

        let head = git_repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        let commit_data = CommitData::from_git2_commit(&commit);

        assert_eq!(commit_data.author.name, "Test User");
        assert_eq!(commit_data.author.email, "test@example.com");
        assert_eq!(commit_data.summary, "Second commit");
        assert!(!commit_data.is_merge);
        assert_eq!(commit_data.parent_ids.len(), 1);
    }

    #[test]
    #[serial]
    fn test_dirty_working_tree_allows_browsing() {
        let (_temp_dir, repo_path) = create_test_repo();

        // Create a modified file
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "modified content").unwrap();

        // Opening should succeed - dirty tree only blocks rewrite, not browsing
        let repo = Repository::open(&repo_path).unwrap();
        assert!(repo.has_uncommitted_changes().unwrap());

        // But validate_clean_for_rewrite should fail
        let result = repo.validate_clean_for_rewrite();
        assert!(matches!(result, Err(HistError::DirtyWorkingTree)));
    }

    #[test]
    #[serial]
    fn test_inner_accessor() {
        let (_temp_dir, repo_path) = create_test_repo();
        let repo = Repository::open(&repo_path).unwrap();

        let inner = repo.inner();
        assert!(!inner.is_bare());
    }

    #[test]
    #[serial]
    fn test_stash_changes_clean_tree() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut repo = Repository::open(&repo_path).unwrap();

        // Clean tree should not stash anything
        let stashed = repo.stash_changes().unwrap();
        assert!(!stashed);
    }

    #[test]
    #[serial]
    fn test_stash_and_unstash_changes() {
        let (_temp_dir, repo_path) = create_test_repo();

        // Create a modified file
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "modified content").unwrap();

        let mut repo = Repository::open(&repo_path).unwrap();
        assert!(repo.has_uncommitted_changes().unwrap());

        // Stash changes
        let stashed = repo.stash_changes().unwrap();
        assert!(stashed);

        // Working tree should now be clean
        assert!(!repo.has_uncommitted_changes().unwrap());

        // Unstash should restore changes
        repo.unstash_changes().unwrap();
        assert!(repo.has_uncommitted_changes().unwrap());

        // File should have modified content
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "modified content");
    }
}
