#[derive(Debug)]
pub enum NodeError {
    AlreadyExists,             // Error jika node dengan nama tersebut sudah ada
    IoError((std::io::Error)), // Menyimpan error dari file system
    JsonError(serde_json::Error),
    NotFound,
    InvalidInput(String), // Untuk input yang tidak valid, seperti nama kosong
    FailedValidation(String), // gagal memvalidasi node
}
