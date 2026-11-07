//! Project settings, with the previous CUBEOPT_* names as fallback aliases.
use std::ffi::OsString;

/// Modern setting names paired with their previous names.
pub const ALIASES: &[(&str, &str)] = &[
    ("EGG_SOLVER_LOAD", "CUBEOPT_LOAD"),
    ("EGG_SOLVER_PREFAULT", "CUBEOPT_PREFAULT"),
    ("EGG_SOLVER_PREFLIGHT_LIMIT", "CUBEOPT_PREFLIGHT_LIMIT"),
    ("EGG_SOLVER_SHARE_PAD", "CUBEOPT_SHARE_PAD"),
    ("EGG_SOLVER_SHM_DENY_HUGE", "CUBEOPT_SHM_DENY_HUGE"),
    ("EGG_SOLVER_SHM_KEY", "CUBEOPT_SHM_KEY"),
    ("EGG_SOLVER_SHM_PAGES", "CUBEOPT_SHM_PAGES"),
    ("EGG_SOLVER_STREAM_PAD", "CUBEOPT_STREAM_PAD"),
    ("EGG_SOLVER_TABLE", "CUBEOPT_TABLE"),
    ("EGG_SOLVER_THREADS", "CUBEOPT_THREADS"),
];

/// Select the modern name when set, otherwise an existing legacy name.
#[must_use]
pub fn name(modern: &'static str) -> &'static str {
    if std::env::var_os(modern).is_some() {
        return modern;
    }
    ALIASES
        .iter()
        .find_map(|&(current, previous)| {
            (current == modern && std::env::var_os(previous).is_some()).then_some(previous)
        })
        .unwrap_or(modern)
}

/// Read a setting, preferring its modern name even when that value is invalid.
///
/// # Errors
/// If the selected variable is absent or is not Unicode.
pub fn var(modern: &str) -> Result<String, std::env::VarError> {
    match std::env::var(modern) {
        Err(std::env::VarError::NotPresent) => {
            if let Some(suffix) = modern.strip_prefix("EGG_SOLVER_") {
                std::env::var(format!("CUBEOPT_{suffix}"))
            } else {
                Err(std::env::VarError::NotPresent)
            }
        }
        result => result,
    }
}

/// Read a possibly non-Unicode setting, with its legacy name as fallback.
#[must_use]
pub fn var_os(modern: &str) -> Option<OsString> {
    std::env::var_os(modern).or_else(|| {
        let suffix = modern.strip_prefix("EGG_SOLVER_")?;
        std::env::var_os(format!("CUBEOPT_{suffix}"))
    })
}

/// Select a compile-time setting without introducing runtime work.
#[must_use]
pub const fn build_var(
    modern: Option<&'static str>,
    previous: Option<&'static str>,
) -> Option<&'static str> {
    match modern {
        Some(value) => Some(value),
        None => previous,
    }
}
