use std::io::Write;

use asset_mapper_core::hash::sha256_file;

#[test]
fn hashes_file_content_with_sha256() {
    let mut file = tempfile::NamedTempFile::new().expect("temp file is created");
    file.write_all(b"asset mapper\n")
        .expect("temp file can be written");

    let hash = sha256_file(file.path()).expect("hash succeeds");

    assert_eq!(
        hash,
        "fdf54baece5b0ff246dc1d2d5b85efc0c51dde8e41013c939648b8a2ac3426a2"
    );
}
