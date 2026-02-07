//! Security utilities for screenpipe-server
//!
//! This module provides security-focused utilities including:
//! - SQL query validation and sanitization
//! - Path traversal protection
//! - Security headers

use axum::http::header::{self, HeaderValue};
use std::path::{Path, PathBuf};
use tower_http::set_header::SetResponseHeaderLayer;

/// Maximum allowed length for SQL queries
const MAX_SQL_QUERY_LENGTH: usize = 10000;

/// Maximum number of rows to return from raw SQL queries
const MAX_SQL_ROWS: usize = 10000;

/// Forbidden SQL keywords for raw SQL endpoint (must be uppercase for comparison)
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "DROP", "DELETE", "UPDATE", "INSERT", "ALTER", "CREATE", "TRUNCATE",
    "REPLACE", "MERGE", "UPSERT", "ATTACH", "DETACH", "PRAGMA", "VACUUM",
    "REINDEX", "ANALYZE", "EXPLAIN", "EXEC", "EXECUTE", "CALL", "LOAD_EXTENSION",
    "COPY", "IMPORT", "EXPORT", "SCRIPT", "SHELL", "SYSTEM",
];

/// SQL injection patterns to detect
const SQL_INJECTION_PATTERNS: &[&str] = &[
    ";--", ";/*", "*/", "@@", "@", "CHAR(", "NCHAR(", "VARCHAR(",
    "NVARCHAR(", "CAST(", "CONVERT(", "HAVING", "GROUP BY", "ORDER BY",
    "UNION", "UNION ALL", "INTERSECT", "EXCEPT", "INTO OUTFILE",
    "INTO DUMPFILE", "LOAD_FILE", "BENCHMARK(", "SLEEP(", "PG_SLEEP(",
    "WAITFOR DELAY", "WAITFOR TIME", "DBMS_PIPE.RECEIVE_MESSAGE",
];

/// Error type for security validation failures
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityError {
    QueryTooLong,
    ForbiddenKeyword(String),
    InjectionDetected(String),
    NotSelectQuery,
    PathTraversal,
    InvalidPath,
    PathOutsideBase,
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::QueryTooLong => write!(f, "SQL query exceeds maximum length"),
            SecurityError::ForbiddenKeyword(kw) => write!(f, "Forbidden SQL keyword detected: {}", kw),
            SecurityError::InjectionDetected(pat) => write!(f, "Potential SQL injection detected: {}", pat),
            SecurityError::NotSelectQuery => write!(f, "Only SELECT queries are allowed"),
            SecurityError::PathTraversal => write!(f, "Path traversal attempt detected"),
            SecurityError::InvalidPath => write!(f, "Invalid path"),
            SecurityError::PathOutsideBase => write!(f, "Path is outside of allowed directory"),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Validates and sanitizes a raw SQL query for safe execution
///
/// # Security checks:
/// 1. Query length limit
/// 2. Must start with SELECT
/// 3. No forbidden keywords (DROP, DELETE, UPDATE, etc.)
/// 4. No common SQL injection patterns
/// 5. No comment sequences that could be used for injection
///
/// # Returns
/// - Ok(()) if the query passes all security checks
/// - Err(SecurityError) if any check fails
pub fn validate_raw_sql_query(query: &str) -> Result<(), SecurityError> {
    // Check query length
    if query.len() > MAX_SQL_QUERY_LENGTH {
        return Err(SecurityError::QueryTooLong);
    }

    // Normalize query for analysis (uppercase, single spaces)
    let normalized = query
        .to_uppercase()
        .replace('\n', " ")
        .replace('\t', " ")
        .replace('\r', " ");

    // Remove excessive spaces for cleaner analysis
    let normalized: String = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Check if query starts with SELECT (after removing leading whitespace)
    let trimmed = normalized.trim_start();
    if !trimmed.starts_with("SELECT ") && !trimmed.starts_with("SELECT\t") && !trimmed.starts_with("SELECT\n") {
        // Also allow WITH for CTEs that contain SELECT
        if !trimmed.starts_with("WITH ") {
            return Err(SecurityError::NotSelectQuery);
        }
    }

    // Check for forbidden keywords
    // We check word boundaries to avoid false positives (e.g., "selection" contains "select")
    for keyword in FORBIDDEN_KEYWORDS {
        // Check for keyword as whole word
        let patterns = [
            format!(" {} ", keyword),  // " KEYWORD "
            format!(" {}(", keyword),  // " KEYWORD("
            format!("\t{}", keyword),  // "\tKEYWORD"
            format!("\n{}", keyword),  // "\nKEYWORD"
        ];

        for pattern in &patterns {
            if normalized.contains(pattern) {
                return Err(SecurityError::ForbiddenKeyword(keyword.to_string()));
            }
        }

        // Check at start of string
        if normalized.starts_with(&format!("{} ", keyword)) {
            return Err(SecurityError::ForbiddenKeyword(keyword.to_string()));
        }
    }

    // Check for SQL injection patterns
    let lower_query = query.to_lowercase();
    for pattern in SQL_INJECTION_PATTERNS {
        if lower_query.contains(&pattern.to_lowercase()) {
            return Err(SecurityError::InjectionDetected(pattern.to_string()));
        }
    }

    // Additional check: ensure no stacked queries (semicolons outside strings)
    // This is a simplified check - in production, consider using a proper SQL parser
    let semicolon_count = query.chars().filter(|&c| c == ';').count();
    if semicolon_count > 1 {
        return Err(SecurityError::InjectionDetected("Multiple statements detected".to_string()));
    }

    Ok(())
}

/// Validates a file path to prevent path traversal attacks
///
/// # Arguments
/// * `path` - The path to validate
/// * `base_dir` - The allowed base directory that the path must be within
///
/// # Returns
/// - Ok(PathBuf) with the canonicalized path if valid
/// - Err(SecurityError) if path traversal is detected
pub fn validate_frame_path(path: &str, base_dir: &Path) -> Result<PathBuf, SecurityError> {
    // Check for null bytes
    if path.contains('\0') {
        return Err(SecurityError::InvalidPath);
    }

    // Parse the path
    let path = Path::new(path);

    // Check for path traversal components
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            // ParentDir (..) is suspicious but not always malicious
            // We'll check canonicalization below
        }
    }

    // Try to canonicalize both paths
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If canonicalization fails, try with base_dir
            let combined = base_dir.join(path);
            match combined.canonicalize() {
                Ok(p) => p,
                Err(_) => return Err(SecurityError::InvalidPath),
            }
        }
    };

    let canonical_base = match base_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(SecurityError::InvalidPath),
    };

    // Ensure the path is within the base directory
    if !canonical_path.starts_with(&canonical_base) {
        return Err(SecurityError::PathOutsideBase);
    }

    Ok(canonical_path)
}

/// Validates that a file path doesn't contain traversal attempts
/// This is a lighter-weight check that doesn't require the file to exist
pub fn validate_path_no_traversal(path: &str) -> Result<(), SecurityError> {
    // Check for null bytes
    if path.contains('\0') {
        return Err(SecurityError::InvalidPath);
    }

    // Check for suspicious patterns
    let suspicious_patterns = ["../", "..\\", "/..", "\\..", "//", "\\\\"];
    for pattern in &suspicious_patterns {
        if path.contains(pattern) {
            return Err(SecurityError::PathTraversal);
        }
    }

    // Check that the path doesn't start with /
    if path.starts_with('/') || path.starts_with("\\") {
        return Err(SecurityError::PathTraversal);
    }

    Ok(())
}

/// Creates a layer that adds security headers to all responses
pub fn create_security_headers_layer() -> SetResponseHeaderLayer<header::HeaderValue> {
    SetResponseHeaderLayer::if_not_present(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    )
}

/// Get the maximum number of rows allowed for raw SQL queries
pub const fn max_sql_rows() -> usize {
    MAX_SQL_ROWS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_select_query() {
        let query = "SELECT * FROM frames WHERE id = 1";
        assert!(validate_raw_sql_query(query).is_ok());
    }

    #[test]
    fn test_valid_select_with_join() {
        let query = "SELECT f.id, f.timestamp FROM frames f JOIN video_chunks v ON f.video_chunk_id = v.id";
        assert!(validate_raw_sql_query(query).is_ok());
    }

    #[test]
    fn test_valid_cte_query() {
        let query = "WITH recent_frames AS (SELECT * FROM frames WHERE timestamp > datetime('now', '-1 day')) SELECT * FROM recent_frames";
        assert!(validate_raw_sql_query(query).is_ok());
    }

    #[test]
    fn test_reject_drop() {
        let query = "DROP TABLE frames";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_reject_delete() {
        let query = "DELETE FROM frames WHERE id = 1";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_reject_update() {
        let query = "UPDATE frames SET app_name = 'test' WHERE id = 1";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_reject_insert() {
        let query = "INSERT INTO frames (id, timestamp) VALUES (1, '2024-01-01')";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_reject_non_select() {
        let query = "PRAGMA table_info(frames)";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::NotSelectQuery) | Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_reject_union_injection() {
        let query = "SELECT * FROM frames UNION SELECT * FROM users";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::InjectionDetected(_))
        ));
    }

    #[test]
    fn test_reject_stacked_queries() {
        let query = "SELECT * FROM frames; DROP TABLE frames;";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::InjectionDetected(_))
        ));
    }

    #[test]
    fn test_case_insensitive_forbidden() {
        let query = "drop table frames";
        assert!(matches!(
            validate_raw_sql_query(query),
            Err(SecurityError::ForbiddenKeyword(_))
        ));
    }

    #[test]
    fn test_path_traversal_detection() {
        assert!(validate_path_no_traversal("../../../etc/passwd").is_err());
        assert!(validate_path_no_traversal("..\\..\\windows\\system32").is_err());
        assert!(validate_path_no_traversal("/etc/passwd").is_err());
        assert!(validate_path_no_traversal("normal/path/to/file.txt").is_ok());
    }

    #[test]
    fn test_query_length_limit() {
        let long_query = "SELECT ".to_string() + &"a".repeat(MAX_SQL_QUERY_LENGTH);
        assert!(matches!(
            validate_raw_sql_query(&long_query),
            Err(SecurityError::QueryTooLong)
        ));
    }
}
