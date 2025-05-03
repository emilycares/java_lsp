pub mod project_kind;

#[derive(Debug, Default, Clone)]
pub struct TaskProgress {
    pub persentage: usize,
    pub error: bool,
    pub message: String,
}
