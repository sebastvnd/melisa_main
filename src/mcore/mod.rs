// melisa core

// STANDART DATE FLOW MCORE
// adapter -> api -> melisad
pub mod adapter; // layer 1 | support semua format dari luar
pub mod api; // layer 2 | filter dan standar format core
pub mod melisad; // layer 3 | melisa deamon/core

pub mod config; // melisa.conf format minimal
pub mod errors; // standar error
pub mod mlog; // log logic from melisa like nginx log

pub mod handler; // menyesuaikankoneksi dari mnode -> adapter
