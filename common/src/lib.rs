#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
pub mod project_kind;

#[derive(Debug, Default, Clone)]
pub struct TaskProgress {
    pub persentage: usize,
    pub error: bool,
    pub message: String,
}
