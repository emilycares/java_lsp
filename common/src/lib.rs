pub mod compile;
pub mod config;
pub mod jdk;
pub mod project_kind;

#[derive(Debug, Default)]
pub struct TaskProgress {
    pub persentage: usize,
    pub message: String,
}
