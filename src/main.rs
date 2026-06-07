mod mcore;

use crate::mcore::adapter::api;
use mcore::services::hashing::get_hash;
use mcore::services::node::NodeProcess;

// Di mulai untuk umat manusia
// Juni 2026
// Kita ke ijen kan?
// Kamu masih ingetkan ...f
fn main() {
    // let name = "Example Pfsfdsroces";
    // let pid = 1243466665;
    // NodeProcess::new(name, pid).expect("Failed to create process");
    // let hashes = "ad39d657c560d67dd124d5568411d7191d46c1671d676f10c7764a74f101f3d1";
    // let d = NodeProcess::delete(hashes);
    // match d {
    //     Ok(_) => println!("Process deleted successfully."),
    //     Err(e) => eprintln!("Failed to delete process: {}", e),
    // }
    let hashes = match get_hash() {
        Some(v) => v,
        None => {
            println!("No processes found.");
            return;
        }
    };
    // 2. Loop menggunakan .enumerate() untuk membuat nomor urut
    for (index, hash) in hashes.iter().enumerate() {
        // index dimulai dari 0, jadi kita tambah 1 agar tampil: 1, 2, 3...
        println!("{}. {}", index + 1, hash);
    }
    // let req = api::fake_data_request();
    // api::execute(&req);
}
