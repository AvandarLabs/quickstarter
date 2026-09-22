//! The command-line surface: the flags a user passes and the questions asked
//! for whatever they left out.

pub mod args;
pub mod prompts;
pub mod resolve;

pub use args::Args;
pub use resolve::Resolver;
