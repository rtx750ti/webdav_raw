use tokio::fs::File;

pub struct FileHandle {
    data: File,
    id: Option<String>,
}
