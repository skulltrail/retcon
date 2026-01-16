use chrono::{DateTime, FixedOffset};
use git2::Oid;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a commit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(#[serde(with = "oid_serde")] pub Oid);

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0.to_string()[..7])
    }
}

/// Serde support for `git2::Oid`
mod oid_serde {
    use git2::Oid;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(oid: &Oid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        oid.to_string().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Oid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Oid::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Represents a person (author or committer)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

impl Person {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }

    /// Format as "Name <email>"
    #[allow(dead_code)]
    #[must_use]
    pub fn format_full(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A commit with all its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    /// Original commit ID
    pub id: CommitId,
    /// Short hash for display (first 7 chars)
    pub short_hash: String,

    // Author information
    pub author: Person,
    pub author_date: DateTime<FixedOffset>,

    // Committer information
    pub committer: Person,
    pub committer_date: DateTime<FixedOffset>,

    // Commit message
    pub message: String,
    /// First line of message for table display
    pub summary: String,

    // Relationships (not editable)
    pub parent_ids: Vec<CommitId>,
    #[serde(with = "oid_serde")]
    pub tree_id: Oid,

    /// Is this a merge commit (multiple parents)?
    pub is_merge: bool,
}

impl CommitData {
    /// Create `CommitData` from a `git2::Commit`
    pub fn from_git2_commit(commit: &git2::Commit<'_>) -> Self {
        let author_sig = commit.author();
        let committer_sig = commit.committer();

        let author = Person::new(
            author_sig.name().unwrap_or("Unknown"),
            author_sig.email().unwrap_or("unknown@example.com"),
        );

        let committer = Person::new(
            committer_sig.name().unwrap_or("Unknown"),
            committer_sig.email().unwrap_or("unknown@example.com"),
        );

        let author_date = git_time_to_datetime(&author_sig.when());
        let committer_date = git_time_to_datetime(&committer_sig.when());

        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();

        let parent_ids: Vec<CommitId> = commit.parent_ids().map(CommitId).collect();
        let is_merge = parent_ids.len() > 1;

        Self {
            id: CommitId(commit.id()),
            short_hash: commit.id().to_string()[..7].to_string(),
            author,
            author_date,
            committer,
            committer_date,
            message,
            summary,
            parent_ids,
            tree_id: commit.tree_id(),
            is_merge,
        }
    }

    /// Get formatted author date for display
    #[must_use]
    pub fn format_author_date(&self) -> String {
        self.author_date.format("%Y-%m-%d %H:%M").to_string()
    }

    /// Get formatted author date with timezone
    #[must_use]
    pub fn format_author_date_full(&self) -> String {
        self.author_date.format("%Y-%m-%d %H:%M:%S %z").to_string()
    }

    /// Get formatted committer date with timezone
    #[must_use]
    pub fn format_committer_date_full(&self) -> String {
        self.committer_date
            .format("%Y-%m-%d %H:%M:%S %z")
            .to_string()
    }
}

/// Convert `git2::Time` to `chrono::DateTime`<FixedOffset>
fn git_time_to_datetime(time: &git2::Time) -> DateTime<FixedOffset> {
    let offset_minutes = time.offset_minutes();
    // UTC (offset 0) is always valid - this cannot fail
    #[allow(clippy::expect_used)]
    let utc = FixedOffset::east_opt(0).expect("UTC offset is always valid");
    let offset = FixedOffset::east_opt(offset_minutes * 60).unwrap_or(utc);
    DateTime::from_timestamp(time.seconds(), 0)
        .unwrap_or_default()
        .with_timezone(&offset)
}

/// Tracks pending modifications to a commit
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitModifications {
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub author_date: Option<DateTime<FixedOffset>>,
    pub committer_name: Option<String>,
    pub committer_email: Option<String>,
    pub committer_date: Option<DateTime<FixedOffset>>,
    pub message: Option<String>,
}

impl CommitModifications {
    /// Check if any modifications have been made
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.author_name.is_none()
            && self.author_email.is_none()
            && self.author_date.is_none()
            && self.committer_name.is_none()
            && self.committer_email.is_none()
            && self.committer_date.is_none()
            && self.message.is_none()
    }

    /// Check if any modifications have been made
    #[must_use]
    pub fn has_modifications(&self) -> bool {
        !self.is_empty()
    }

    /// Get the effective author name (modified or original)
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_author_name<'a>(&'a self, original: &'a str) -> &'a str {
        self.author_name.as_deref().unwrap_or(original)
    }

    /// Get the effective author email (modified or original)
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_author_email<'a>(&'a self, original: &'a str) -> &'a str {
        self.author_email.as_deref().unwrap_or(original)
    }

    /// Get the effective committer name (modified or original)
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_committer_name<'a>(&'a self, original: &'a str) -> &'a str {
        self.committer_name.as_deref().unwrap_or(original)
    }

    /// Get the effective committer email (modified or original)
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_committer_email<'a>(&'a self, original: &'a str) -> &'a str {
        self.committer_email.as_deref().unwrap_or(original)
    }

    /// Get the effective message (modified or original)
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_message<'a>(&'a self, original: &'a str) -> &'a str {
        self.message.as_deref().unwrap_or(original)
    }

    /// Get summary from effective message
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_summary<'a>(&'a self, original: &'a str) -> &'a str {
        self.message
            .as_deref()
            .map_or(original, |m| m.lines().next().unwrap_or(""))
    }

    /// Count how many fields have been modified
    #[allow(dead_code)]
    #[must_use]
    pub fn modification_count(&self) -> usize {
        let mut count = 0;
        if self.author_name.is_some() {
            count += 1;
        }
        if self.author_email.is_some() {
            count += 1;
        }
        if self.author_date.is_some() {
            count += 1;
        }
        if self.committer_name.is_some() {
            count += 1;
        }
        if self.committer_email.is_some() {
            count += 1;
        }
        if self.committer_date.is_some() {
            count += 1;
        }
        if self.message.is_some() {
            count += 1;
        }
        count
    }
}

/// Fields that can be edited on a commit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditableField {
    AuthorName,
    AuthorEmail,
    AuthorDate,
    CommitterName,
    CommitterEmail,
    CommitterDate,
    Message,
}

impl EditableField {
    /// Get all editable fields in order
    #[allow(dead_code)]
    #[must_use]
    pub fn all() -> &'static [EditableField] {
        &[
            EditableField::AuthorName,
            EditableField::AuthorEmail,
            EditableField::AuthorDate,
            EditableField::CommitterName,
            EditableField::CommitterEmail,
            EditableField::CommitterDate,
            EditableField::Message,
        ]
    }

    /// Get display name for the field
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            EditableField::AuthorName => "Author Name",
            EditableField::AuthorEmail => "Author Email",
            EditableField::AuthorDate => "Author Date",
            EditableField::CommitterName => "Committer Name",
            EditableField::CommitterEmail => "Committer Email",
            EditableField::CommitterDate => "Committer Date",
            EditableField::Message => "Commit Message",
        }
    }

    /// Get short label for table columns
    #[allow(dead_code)]
    #[must_use]
    pub fn short_label(&self) -> &'static str {
        match self {
            EditableField::AuthorName => "Author",
            EditableField::AuthorEmail => "Email",
            EditableField::AuthorDate => "Date",
            EditableField::CommitterName => "Committer",
            EditableField::CommitterEmail => "C.Email",
            EditableField::CommitterDate => "C.Date",
            EditableField::Message => "Message",
        }
    }

    /// Get next field (for Tab navigation)
    #[allow(dead_code)]
    #[must_use]
    pub fn next(&self) -> EditableField {
        match self {
            EditableField::AuthorName => EditableField::AuthorEmail,
            EditableField::AuthorEmail => EditableField::AuthorDate,
            EditableField::AuthorDate => EditableField::CommitterName,
            EditableField::CommitterName => EditableField::CommitterEmail,
            EditableField::CommitterEmail => EditableField::CommitterDate,
            EditableField::CommitterDate => EditableField::Message,
            EditableField::Message => EditableField::AuthorName,
        }
    }

    /// Get previous field (for Shift+Tab navigation)
    #[allow(dead_code)]
    #[must_use]
    pub fn prev(&self) -> EditableField {
        match self {
            EditableField::AuthorName => EditableField::Message,
            EditableField::AuthorEmail => EditableField::AuthorName,
            EditableField::AuthorDate => EditableField::AuthorEmail,
            EditableField::CommitterName => EditableField::AuthorDate,
            EditableField::CommitterEmail => EditableField::CommitterName,
            EditableField::CommitterDate => EditableField::CommitterEmail,
            EditableField::Message => EditableField::CommitterDate,
        }
    }

    /// Is this a date field?
    #[must_use]
    pub fn is_date(&self) -> bool {
        matches!(
            self,
            EditableField::AuthorDate | EditableField::CommitterDate
        )
    }

    /// Is this an email field?
    #[must_use]
    pub fn is_email(&self) -> bool {
        matches!(
            self,
            EditableField::AuthorEmail | EditableField::CommitterEmail
        )
    }

    /// Is this a multiline field?
    #[allow(dead_code)]
    #[must_use]
    pub fn is_multiline(&self) -> bool {
        matches!(self, EditableField::Message)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_commit_id_display() {
        let oid = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let id = CommitId(oid);
        assert_eq!(id.to_string(), "1234567");
    }

    #[test]
    fn test_commit_id_equality() {
        let oid1 = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let oid2 = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let oid3 = git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap();

        assert_eq!(CommitId(oid1), CommitId(oid2));
        assert_ne!(CommitId(oid1), CommitId(oid3));
    }

    #[test]
    fn test_person_creation() {
        let person = Person::new("John Doe", "john@example.com");
        assert_eq!(person.name, "John Doe");
        assert_eq!(person.email, "john@example.com");
    }

    #[test]
    fn test_person_format_full() {
        let person = Person::new("Jane Smith", "jane@example.com");
        assert_eq!(person.format_full(), "Jane Smith <jane@example.com>");
    }

    #[test]
    fn test_person_display() {
        let person = Person::new("Bob", "bob@example.com");
        assert_eq!(person.to_string(), "Bob");
    }

    #[test]
    fn test_commit_modifications_is_empty() {
        let mods = CommitModifications::default();
        assert!(mods.is_empty());
        assert!(!mods.has_modifications());
    }

    #[test]
    fn test_commit_modifications_with_author_name() {
        let mods = CommitModifications {
            author_name: Some("New Author".to_string()),
            ..Default::default()
        };
        assert!(!mods.is_empty());
        assert!(mods.has_modifications());
        assert_eq!(mods.modification_count(), 1);
    }

    #[test]
    fn test_commit_modifications_with_multiple_fields() {
        let mods = CommitModifications {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            message: Some("New message".to_string()),
            ..Default::default()
        };

        assert_eq!(mods.modification_count(), 3);
        assert!(mods.has_modifications());
    }

    #[test]
    fn test_commit_modifications_effective_values() {
        let mods = CommitModifications {
            author_name: Some("Modified".to_string()),
            ..Default::default()
        };

        assert_eq!(mods.effective_author_name("Original"), "Modified");
        assert_eq!(
            mods.effective_author_email("original@test.com"),
            "original@test.com"
        );
    }

    #[test]
    fn test_commit_modifications_effective_message() {
        let mods = CommitModifications {
            message: Some("New message\nSecond line".to_string()),
            ..Default::default()
        };

        assert_eq!(mods.effective_message("Old"), "New message\nSecond line");
        assert_eq!(mods.effective_summary("Old summary"), "New message");
    }

    #[test]
    fn test_commit_modifications_effective_summary_empty_line() {
        let mods = CommitModifications {
            message: Some("\nSecond line".to_string()),
            ..Default::default()
        };

        // First line is empty, so summary should be empty
        assert_eq!(mods.effective_summary("Old summary"), "");
    }

    #[test]
    fn test_editable_field_display_name() {
        assert_eq!(EditableField::AuthorName.display_name(), "Author Name");
        assert_eq!(EditableField::AuthorEmail.display_name(), "Author Email");
        assert_eq!(EditableField::AuthorDate.display_name(), "Author Date");
        assert_eq!(
            EditableField::CommitterName.display_name(),
            "Committer Name"
        );
        assert_eq!(
            EditableField::CommitterEmail.display_name(),
            "Committer Email"
        );
        assert_eq!(
            EditableField::CommitterDate.display_name(),
            "Committer Date"
        );
        assert_eq!(EditableField::Message.display_name(), "Commit Message");
    }

    #[test]
    fn test_editable_field_short_label() {
        assert_eq!(EditableField::AuthorName.short_label(), "Author");
        assert_eq!(EditableField::AuthorEmail.short_label(), "Email");
        assert_eq!(EditableField::CommitterEmail.short_label(), "C.Email");
    }

    #[test]
    fn test_editable_field_navigation() {
        let field = EditableField::AuthorName;
        assert_eq!(field.next(), EditableField::AuthorEmail);
        assert_eq!(field.next().next(), EditableField::AuthorDate);
        assert_eq!(field.prev(), EditableField::Message);
    }

    #[test]
    fn test_editable_field_navigation_wraps() {
        // Test that next wraps from Message to AuthorName
        assert_eq!(EditableField::Message.next(), EditableField::AuthorName);
        // Test that prev wraps from AuthorName to Message
        assert_eq!(EditableField::AuthorName.prev(), EditableField::Message);
    }

    #[test]
    fn test_editable_field_is_date() {
        assert!(EditableField::AuthorDate.is_date());
        assert!(EditableField::CommitterDate.is_date());
        assert!(!EditableField::AuthorName.is_date());
        assert!(!EditableField::Message.is_date());
    }

    #[test]
    fn test_editable_field_is_email() {
        assert!(EditableField::AuthorEmail.is_email());
        assert!(EditableField::CommitterEmail.is_email());
        assert!(!EditableField::AuthorName.is_email());
        assert!(!EditableField::Message.is_email());
    }

    #[test]
    fn test_editable_field_is_multiline() {
        assert!(EditableField::Message.is_multiline());
        assert!(!EditableField::AuthorName.is_multiline());
        assert!(!EditableField::AuthorDate.is_multiline());
    }

    #[test]
    fn test_editable_field_all() {
        let all = EditableField::all();
        assert_eq!(all.len(), 7);
        assert_eq!(all[0], EditableField::AuthorName);
        assert_eq!(all[6], EditableField::Message);
    }

    #[test]
    fn test_git_time_to_datetime() {
        use git2::Time;

        // Create a git time: Jan 15, 2024 14:30:00 UTC
        let git_time = Time::new(1_705_330_200, 0);
        let dt = super::git_time_to_datetime(&git_time);

        assert_eq!(dt.timestamp(), 1_705_330_200);
        assert_eq!(dt.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_git_time_to_datetime_with_offset() {
        use git2::Time;

        // Create a git time with +05:30 offset (330 minutes)
        let git_time = Time::new(1_705_330_200, 330);
        let dt = super::git_time_to_datetime(&git_time);

        assert_eq!(dt.timestamp(), 1_705_330_200);
        assert_eq!(dt.offset().local_minus_utc(), 330 * 60);
    }

    #[test]
    fn test_commit_data_format_dates() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        let commit = CommitData {
            id: CommitId(git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap()),
            short_hash: "1234567".to_string(),
            author: Person::new("Test Author", "test@example.com"),
            author_date: dt,
            committer: Person::new("Test Committer", "commit@example.com"),
            committer_date: dt,
            message: "Test commit".to_string(),
            summary: "Test commit".to_string(),
            parent_ids: vec![],
            tree_id: git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap(),
            is_merge: false,
        };

        assert_eq!(commit.format_author_date(), "2024-01-15 14:30");
        assert_eq!(
            commit.format_author_date_full(),
            "2024-01-15 14:30:00 +0000"
        );
        assert_eq!(
            commit.format_committer_date_full(),
            "2024-01-15 14:30:00 +0000"
        );
    }

    #[test]
    fn test_commit_data_is_merge() {
        let oid1 = git2::Oid::from_str("1111111111111111111111111111111111111111").unwrap();
        let oid2 = git2::Oid::from_str("2222222222222222222222222222222222222222").unwrap();
        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        // Regular commit (no parents)
        let regular = CommitData {
            id: CommitId(oid1),
            short_hash: "1111111".to_string(),
            author: Person::new("Test", "test@example.com"),
            author_date: dt,
            committer: Person::new("Test", "test@example.com"),
            committer_date: dt,
            message: "Test".to_string(),
            summary: "Test".to_string(),
            parent_ids: vec![],
            tree_id: oid2,
            is_merge: false,
        };
        assert!(!regular.is_merge);

        // Merge commit (two parents)
        let merge = CommitData {
            id: CommitId(oid1),
            short_hash: "1111111".to_string(),
            author: Person::new("Test", "test@example.com"),
            author_date: dt,
            committer: Person::new("Test", "test@example.com"),
            committer_date: dt,
            message: "Test".to_string(),
            summary: "Test".to_string(),
            parent_ids: vec![CommitId(oid1), CommitId(oid2)],
            tree_id: oid2,
            is_merge: true,
        };
        assert!(merge.is_merge);
    }

    #[test]
    fn test_commit_id_serde() {
        let oid = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let id = CommitId(oid);

        // Serialize
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("1234567890abcdef1234567890abcdef12345678"));

        // Deserialize
        let deserialized: CommitId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_person_equality() {
        let p1 = Person::new("John", "john@example.com");
        let p2 = Person::new("John", "john@example.com");
        let p3 = Person::new("Jane", "jane@example.com");

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }
}
