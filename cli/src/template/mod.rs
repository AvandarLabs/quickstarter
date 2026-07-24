//! Fetching the template repository at runtime.
//!
//! The binary ships no templates of its own. It clones the template repo fresh
//! on every run into a temporary directory, so even an old binary always
//! builds from the newest templates, and the user never sees the clone or any
//! intermediate files: the temp directory is removed when the run ends.

pub mod fetch;

pub use fetch::{TemplateCheckout, ensure_git_available};

/// The default template repository, cloned when the user does not pass a
/// `--repo` override. See the repository README for the canonical location.
pub const DEFAULT_TEMPLATE_REPO: &str = "https://github.com/AvandarLabs/quickstarter.git";
