//! Session memory for coding-agent harnesses.
//!
//! The engine stores an append-only session log and a small user directory.
//! It does not call a model. The caller decides whether long-term memory is
//! opened, and supplies any prose its own model wrote.

mod classify;
mod engine;
mod error;
mod redact;
mod store;

pub use classify::Kind;
pub use engine::{
    default_user_home, Append, Appended, Artifact, Engine, RecallPack, RecallQuery, ReflectLong,
    ReflectMid, DEFAULT_RECENT,
};
pub use error::{Error, Result};
pub use store::{LongTerm, MidTerm, Note, Slot, StoredEvent};
