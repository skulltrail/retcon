use crate::error::{HistError, Result};
use crate::git::commit::{CommitData, CommitId, CommitModifications};
use chrono::{DateTime, FixedOffset};
use git2::{Repository as Git2Repository, Signature, Time};
use std::collections::HashMap;

/// Rewrite git history with the specified modifications
///
/// This function rewrites commits from oldest to newest, creating new commits
/// with the modified metadata while preserving the tree (file contents).
///
/// # Arguments
/// * `repo` - The git repository
/// * `commits` - List of commits in display order (newest first)
/// * `modifications` - Map of commit ID to modifications
/// * `new_order` - New order of commits (for reordering support)
/// * `branch_name` - Name of the branch to update
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(HistError)` on failure
pub fn rewrite_history(
    repo: &Git2Repository,
    commits: &[CommitData],
    modifications: &HashMap<CommitId, CommitModifications>,
    new_order: &[CommitId],
    branch_name: &str,
) -> Result<()> {
    // Build a lookup map for commits by ID
    let commit_lookup: HashMap<CommitId, &CommitData> = commits.iter().map(|c| (c.id, c)).collect();

    // Map from old commit OID to new commit OID
    let mut commit_map: HashMap<git2::Oid, git2::Oid> = HashMap::new();

    // Process commits from oldest to newest (reverse of display order)
    for commit_id in new_order.iter().rev() {
        let original = commit_lookup
            .get(commit_id)
            .ok_or_else(|| HistError::CommitNotFound(commit_id.to_string()))?;

        let mods = modifications.get(commit_id);

        // Get parent commits, translating through commit_map if they were rewritten
        let parent_oids: Vec<git2::Oid> = original
            .parent_ids
            .iter()
            .map(|p| *commit_map.get(&p.0).unwrap_or(&p.0))
            .collect();

        let parents: Vec<git2::Commit<'_>> = parent_oids
            .iter()
            .map(|oid| repo.find_commit(*oid))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

        // Build author signature
        let author = build_signature(
            mods.and_then(|m| m.author_name.as_deref())
                .unwrap_or(&original.author.name),
            mods.and_then(|m| m.author_email.as_deref())
                .unwrap_or(&original.author.email),
            mods.and_then(|m| m.author_date)
                .unwrap_or(original.author_date),
        )?;

        // Build committer signature
        let committer = build_signature(
            mods.and_then(|m| m.committer_name.as_deref())
                .unwrap_or(&original.committer.name),
            mods.and_then(|m| m.committer_email.as_deref())
                .unwrap_or(&original.committer.email),
            mods.and_then(|m| m.committer_date)
                .unwrap_or(original.committer_date),
        )?;

        // Get the message
        let message = mods
            .and_then(|m| m.message.as_deref())
            .unwrap_or(&original.message);

        // Get the original tree (file contents unchanged)
        let tree = repo.find_tree(original.tree_id)?;

        // Create the new commit
        let new_oid = repo.commit(
            None, // Don't update any ref yet
            &author,
            &committer,
            message,
            &tree,
            &parent_refs,
        )?;

        // Record the mapping
        commit_map.insert(original.id.0, new_oid);
    }

    // Update the branch reference to point to the new HEAD
    let newest_commit_id = new_order
        .first()
        .ok_or_else(|| HistError::RewriteFailed("No commits to rewrite".to_string()))?;

    let new_head_oid = commit_map
        .get(&newest_commit_id.0)
        .ok_or_else(|| HistError::RewriteFailed("Failed to find new HEAD commit".to_string()))?;

    // Update the branch reference
    let ref_name = format!("refs/heads/{}", branch_name);
    repo.reference(
        &ref_name,
        *new_head_oid,
        true, // Force update
        "retcon: rewrite history",
    )?;

    // Also update HEAD if it's pointing to this branch
    if let Ok(head) = repo.head() {
        if head.is_branch() && head.shorthand() == Some(branch_name) {
            // HEAD will automatically follow the branch
        }
    }

    Ok(())
}

/// Build a git2 Signature from name, email, and datetime
fn build_signature(
    name: &str,
    email: &str,
    datetime: DateTime<FixedOffset>,
) -> Result<Signature<'static>> {
    let time = datetime_to_git_time(&datetime);
    Signature::new(name, email, &time).map_err(|e| HistError::Git(e))
}

/// Convert chrono DateTime to git2 Time
fn datetime_to_git_time(dt: &DateTime<FixedOffset>) -> Time {
    let offset_minutes = dt.offset().local_minus_utc() / 60;
    Time::new(dt.timestamp(), offset_minutes as i32)
}

/// Check if any commits have been modified
#[allow(dead_code)]
pub fn has_modifications(modifications: &HashMap<CommitId, CommitModifications>) -> bool {
    modifications.values().any(|m| m.has_modifications())
}

/// Check if the commit order has changed
pub fn order_changed(original_order: &[CommitId], new_order: &[CommitId]) -> bool {
    if original_order.len() != new_order.len() {
        return true;
    }
    original_order
        .iter()
        .zip(new_order.iter())
        .any(|(a, b)| a != b)
}

/// Count total number of modified commits
pub fn count_modified_commits(modifications: &HashMap<CommitId, CommitModifications>) -> usize {
    modifications
        .values()
        .filter(|m| m.has_modifications())
        .count()
}

/// Generate a summary of changes for the confirmation dialog
pub fn generate_change_summary(
    commits: &[CommitData],
    modifications: &HashMap<CommitId, CommitModifications>,
    original_order: &[CommitId],
    new_order: &[CommitId],
) -> Vec<String> {
    let mut summary = Vec::new();

    // Count modified commits
    let modified_count = count_modified_commits(modifications);
    if modified_count > 0 {
        summary.push(format!(
            "{} commit(s) with modified metadata",
            modified_count
        ));
    }

    // Check for reordering
    if order_changed(original_order, new_order) {
        summary.push("Commit order has been changed".to_string());
    }

    // List specific changes per commit
    for commit in commits.iter().take(5) {
        if let Some(mods) = modifications.get(&commit.id) {
            if mods.has_modifications() {
                let mut changes = Vec::new();
                if mods.author_name.is_some() {
                    changes.push("author name");
                }
                if mods.author_email.is_some() {
                    changes.push("author email");
                }
                if mods.author_date.is_some() {
                    changes.push("author date");
                }
                if mods.committer_name.is_some() {
                    changes.push("committer name");
                }
                if mods.committer_email.is_some() {
                    changes.push("committer email");
                }
                if mods.committer_date.is_some() {
                    changes.push("committer date");
                }
                if mods.message.is_some() {
                    changes.push("message");
                }

                summary.push(format!("  {} - {}", commit.short_hash, changes.join(", ")));
            }
        }
    }

    if modified_count > 5 {
        summary.push(format!("  ... and {} more", modified_count - 5));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_modifications_empty() {
        let mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        assert!(!has_modifications(&mods));
    }

    #[test]
    fn test_order_changed() {
        use git2::Oid;
        let id1 = CommitId(Oid::from_str("1111111111111111111111111111111111111111").unwrap());
        let id2 = CommitId(Oid::from_str("2222222222222222222222222222222222222222").unwrap());

        assert!(!order_changed(&[id1, id2], &[id1, id2]));
        assert!(order_changed(&[id1, id2], &[id2, id1]));
        assert!(order_changed(&[id1], &[id1, id2]));
    }

    #[test]
    fn test_count_modified_commits() {
        let mut mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        let id1 = CommitId(git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap());
        let id2 = CommitId(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap());

        // No modifications
        assert_eq!(count_modified_commits(&mods), 0);

        // Add an empty modification (should not count)
        mods.insert(id1, CommitModifications::default());
        assert_eq!(count_modified_commits(&mods), 0);

        // Add a real modification
        let mut mod1 = CommitModifications::default();
        mod1.author_name = Some("New Author".to_string());
        mods.insert(id1, mod1);
        assert_eq!(count_modified_commits(&mods), 1);

        // Add another modification
        let mut mod2 = CommitModifications::default();
        mod2.message = Some("New message".to_string());
        mods.insert(id2, mod2);
        assert_eq!(count_modified_commits(&mods), 2);
    }

    #[test]
    fn test_generate_change_summary_no_changes() {
        let commits = vec![];
        let mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        let order1 = vec![];
        let order2 = vec![];

        let summary = generate_change_summary(&commits, &mods, &order1, &order2);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_generate_change_summary_with_modifications() {
        use chrono::{FixedOffset, TimeZone};

        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        let id1 = CommitId(git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap());
        let commit = crate::git::commit::CommitData {
            id: id1,
            short_hash: "1111111".to_string(),
            author: crate::git::commit::Person::new("Test", "test@example.com"),
            author_date: dt,
            committer: crate::git::commit::Person::new("Test", "test@example.com"),
            committer_date: dt,
            message: "Test".to_string(),
            summary: "Test".to_string(),
            parent_ids: vec![],
            tree_id: git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap(),
            is_merge: false,
        };

        let mut mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        let mut mod1 = CommitModifications::default();
        mod1.author_name = Some("New Author".to_string());
        mod1.author_email = Some("new@example.com".to_string());
        mods.insert(id1, mod1);

        let summary = generate_change_summary(&[commit], &mods, &[id1], &[id1]);

        assert!(summary.len() >= 2);
        assert!(summary[0].contains("1 commit(s) with modified metadata"));
        assert!(summary[1].contains("1111111"));
        assert!(summary[1].contains("author name"));
        assert!(summary[1].contains("author email"));
    }

    #[test]
    fn test_generate_change_summary_with_reorder() {
        let id1 = CommitId(git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap());
        let id2 = CommitId(git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap());

        let commits = vec![];
        let mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        let original_order = vec![id1, id2];
        let new_order = vec![id2, id1];

        let summary = generate_change_summary(&commits, &mods, &original_order, &new_order);

        assert_eq!(summary.len(), 1);
        assert!(summary[0].contains("Commit order has been changed"));
    }

    #[test]
    fn test_generate_change_summary_many_commits() {
        use chrono::{FixedOffset, TimeZone};

        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        // Create 10 commits
        let commits: Vec<_> = (0..10)
            .map(|i| {
                let id_str = format!("{}111111111111111111111111111111111111", i);
                let oid = git2::Oid::from_str(&id_str).unwrap();
                crate::git::commit::CommitData {
                    id: CommitId(oid),
                    short_hash: id_str[..7].to_string(),
                    author: crate::git::commit::Person::new("Test", "test@example.com"),
                    author_date: dt,
                    committer: crate::git::commit::Person::new("Test", "test@example.com"),
                    committer_date: dt,
                    message: format!("Commit {}", i),
                    summary: format!("Commit {}", i),
                    parent_ids: vec![],
                    tree_id: git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap(),
                    is_merge: false,
                }
            })
            .collect();

        // Modify all commits
        let mut mods: HashMap<CommitId, CommitModifications> = HashMap::new();
        for commit in &commits {
            let mut mod1 = CommitModifications::default();
            mod1.message = Some("Modified".to_string());
            mods.insert(commit.id, mod1);
        }

        let order: Vec<_> = commits.iter().map(|c| c.id).collect();
        let summary = generate_change_summary(&commits, &mods, &order, &order);

        // Should show first 5 and then "... and X more"
        assert!(summary.iter().any(|s| s.contains("... and 5 more")));
    }

    #[test]
    fn test_datetime_to_git_time() {
        use chrono::{FixedOffset, TimeZone};

        let offset = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
        let dt = offset.with_ymd_and_hms(2024, 1, 15, 14, 30, 45).unwrap();

        let git_time = super::datetime_to_git_time(&dt);

        assert_eq!(git_time.seconds(), dt.timestamp());
        assert_eq!(git_time.offset_minutes(), (5 * 60 + 30) as i32);
    }

    #[test]
    fn test_datetime_to_git_time_negative_offset() {
        use chrono::{FixedOffset, TimeZone};

        let offset = FixedOffset::west_opt(8 * 3600).unwrap();
        let dt = offset.with_ymd_and_hms(2024, 1, 15, 14, 30, 45).unwrap();

        let git_time = super::datetime_to_git_time(&dt);

        assert_eq!(git_time.seconds(), dt.timestamp());
        assert_eq!(git_time.offset_minutes(), -(8 * 60) as i32);
    }

    #[test]
    fn test_build_signature() {
        use chrono::{FixedOffset, TimeZone};

        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        let sig = super::build_signature("Test User", "test@example.com", dt).unwrap();

        assert_eq!(sig.name(), Some("Test User"));
        assert_eq!(sig.email(), Some("test@example.com"));
        assert_eq!(sig.when().seconds(), dt.timestamp());
    }
}
