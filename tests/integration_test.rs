use retcon::{git::Repository, state::app_state::AppState, Result};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

/// Helper to create a test git repository with multiple commits
fn create_test_repo_with_commits(
    commits: &[(&str, &str)],
) -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize repository
    let repo = git2::Repository::init(&repo_path).unwrap();

    // Configure user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    let sig = git2::Signature::now("Test User", "test@example.com").unwrap();

    for (i, (filename, message)) in commits.iter().enumerate() {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, format!("Content {}", i)).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        if i == 0 {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap();
        } else {
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .unwrap();
        }
    }

    (temp_dir, repo_path)
}

#[test]
#[serial]
fn test_repository_workflow() -> Result<()> {
    let commits = vec![
        ("file1.txt", "Initial commit"),
        ("file2.txt", "Add file2"),
        ("file3.txt", "Add file3"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits);

    // Open repository
    let repo = Repository::open(&repo_path)?;

    // Load commits
    let commits = repo.load_commits(10)?;
    assert_eq!(commits.len(), 3);

    // Verify order (newest first)
    assert_eq!(commits[0].summary, "Add file3");
    assert_eq!(commits[1].summary, "Add file2");
    assert_eq!(commits[2].summary, "Initial commit");

    Ok(())
}

#[test]
#[serial]
fn test_app_state_workflow() -> Result<()> {
    let commits = vec![
        ("file1.txt", "First"),
        ("file2.txt", "Second"),
        ("file3.txt", "Third"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits);
    let repo = Repository::open(&repo_path)?;
    let loaded_commits = repo.load_commits(10)?;
    let branch_name = repo.current_branch_name()?;
    let has_upstream = repo.has_upstream()?;

    // Create app state
    let mut state = AppState::new(loaded_commits, branch_name, has_upstream);

    // Test modifications
    let commit_id = state.commits[0].id;
    let mods = state.get_or_create_modifications(commit_id);
    mods.author_name = Some("New Author".to_string());

    assert!(state.is_dirty());
    assert_eq!(state.modified_count(), 1);

    Ok(())
}

#[test]
#[serial]
fn test_commit_rewriting() -> Result<()> {
    use retcon::git::commit::CommitModifications;
    use retcon::git::rewrite::rewrite_history;
    use std::collections::HashMap;

    let commits_data = vec![
        ("file1.txt", "First"),
        ("file2.txt", "Second"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;
    let commits = repo.load_commits(10)?;
    let branch_name = repo.current_branch_name()?;

    // Create modifications for the first commit
    let mut modifications = HashMap::new();
    let mut mod1 = CommitModifications::default();
    mod1.author_name = Some("Modified Author".to_string());
    mod1.message = Some("Modified message".to_string());
    modifications.insert(commits[0].id, mod1);

    // Get the current order
    let current_order: Vec<_> = commits.iter().map(|c| c.id).collect();

    // Rewrite history
    rewrite_history(
        repo.inner(),
        &commits,
        &modifications,
        &current_order,
        &branch_name,
    )?;

    // Reopen and verify changes
    let repo2 = Repository::open(&repo_path)?;
    let new_commits = repo2.load_commits(10)?;

    // The commit IDs should be different (new commits were created)
    assert_ne!(new_commits[0].id, commits[0].id);

    // But the author name and message should be modified
    assert_eq!(new_commits[0].author.name, "Modified Author");
    assert_eq!(new_commits[0].message, "Modified message");

    // The second commit should also have a new ID (because its parent's ID changed)
    // Note: In git, changing a commit creates a new tree of commits from that point forward
    // However, if only the first commit was modified, the second commit might not change
    // if git optimizes and sees the tree is identical
    // For now, we just verify the first commit changed
    assert_eq!(new_commits.len(), 2);

    Ok(())
}

#[test]
#[serial]
fn test_validation_integration() -> Result<()> {
    use retcon::git::validation::{validate_date, validate_email};

    // Test email validation
    assert!(validate_email("user@example.com").is_ok());
    assert!(validate_email("invalid").is_err());

    // Test date validation
    assert!(validate_date("2024-01-15 14:30:00 +0000").is_ok());
    assert!(validate_date("invalid-date").is_err());

    Ok(())
}

#[test]
#[serial]
fn test_filter_commits() -> Result<()> {
    let commits_data = vec![
        ("file1.txt", "Add authentication"),
        ("file2.txt", "Fix bug in parser"),
        ("file3.txt", "Add tests for authentication"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;
    let commits = repo.load_commits(10)?;
    let branch_name = repo.current_branch_name()?;
    let has_upstream = repo.has_upstream()?;

    let mut state = AppState::new(commits, branch_name, has_upstream);

    // Filter for "authentication"
    state.search_query = "authentication".to_string();
    state.apply_filter();

    let visible = state.visible_commits();
    assert_eq!(visible.len(), 2);

    // Clear filter
    state.clear_filter();
    let visible = state.visible_commits();
    assert_eq!(visible.len(), 3);

    Ok(())
}

#[test]
#[serial]
fn test_commit_selection() -> Result<()> {
    let commits_data = vec![
        ("file1.txt", "First"),
        ("file2.txt", "Second"),
        ("file3.txt", "Third"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;
    let commits = repo.load_commits(10)?;
    let branch_name = repo.current_branch_name()?;
    let has_upstream = repo.has_upstream()?;

    let mut state = AppState::new(commits, branch_name, has_upstream);

    // Select all commits
    state.select_all();
    assert_eq!(state.selected.len(), 3);

    // Deselect all
    state.deselect_all();
    assert_eq!(state.selected.len(), 0);

    // Toggle individual selection
    state.toggle_selection();
    assert_eq!(state.selected.len(), 1);

    Ok(())
}

#[test]
#[serial]
fn test_undo_redo_integration() -> Result<()> {
    let commits_data = vec![("file1.txt", "First")];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;
    let commits = repo.load_commits(10)?;
    let branch_name = repo.current_branch_name()?;
    let has_upstream = repo.has_upstream()?;

    let mut state = AppState::new(commits, branch_name, has_upstream);
    let commit_id = state.commits[0].id;

    // Save undo point
    state.save_undo("test modification");

    // Make modification
    let mods = state.get_or_create_modifications(commit_id);
    mods.author_name = Some("New Name".to_string());

    assert!(state.is_dirty());

    // Undo
    state.undo();
    assert!(!state.is_dirty());

    // Redo
    state.redo();
    assert!(state.is_dirty());

    Ok(())
}

#[test]
#[serial]
fn test_backup_ref_creation() -> Result<()> {
    let commits_data = vec![("file1.txt", "First")];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;

    // Create backup
    repo.create_backup_ref("main")?;

    // Verify backup exists
    let git_repo = repo.inner();
    assert!(git_repo
        .find_reference("refs/original/heads/main")
        .is_ok());

    Ok(())
}

#[test]
#[serial]
fn test_dirty_working_tree_detection() {
    let commits_data = vec![("file1.txt", "First")];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);

    // Modify a file to make working tree dirty
    let file_path = repo_path.join("file1.txt");
    fs::write(&file_path, "Modified content").unwrap();

    // Should fail to open due to dirty working tree
    let result = Repository::open(&repo_path);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_commit_count() -> Result<()> {
    let commits_data = vec![
        ("file1.txt", "First"),
        ("file2.txt", "Second"),
        ("file3.txt", "Third"),
        ("file4.txt", "Fourth"),
        ("file5.txt", "Fifth"),
    ];

    let (_temp_dir, repo_path) = create_test_repo_with_commits(&commits_data);
    let repo = Repository::open(&repo_path)?;

    let count = repo.commit_count()?;
    assert_eq!(count, 5);

    // Load with limit
    let commits = repo.load_commits(3)?;
    assert_eq!(commits.len(), 3);

    Ok(())
}
