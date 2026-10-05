#[derive(Debug, Clone)]
pub struct Baseline {
    pub test_count: u32,
    pub skipped: u32,
    pub trusted_names: Vec<String>,
}
