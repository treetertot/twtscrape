#[derive(Debug, Clone)]
pub struct Query {
    pub mode: SearchMode,
    pub include_replies: bool,
    pub batch_size: usize,
    pub user: String,
}
// Maybe also make seperate functions
// Move to query module
#[derive(Debug, Clone)]
pub enum SearchMode {
    Top,
    Latest,
    Photos,
    Videos,
    Users,
}
