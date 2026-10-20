use pcl_download_patch::write_atomic;
use std::fs;

#[test]
fn write_atomic_roundtrip() {
    let dir = std::env::temp_dir().join(format!("pclpatch-io-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let target = dir.join("target.bin");
    let payload: Vec<u8> = (0..=255u8).cycle().take(65536).collect();
    write_atomic(&target, &payload).unwrap();
    assert_eq!(fs::read(&target).unwrap(), payload);
    assert!(!dir.join("target.bin.tmp").exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn write_atomic_overwrites_existing() {
    let dir = std::env::temp_dir().join(format!("pclpatch-io2-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let target = dir.join("target.bin");
    fs::write(&target, b"old").unwrap();
    write_atomic(&target, b"new").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"new");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn write_atomic_cleans_tmp_on_failure() {
    let dir = std::env::temp_dir().join(format!("pclpatch-io3-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let target_is_dir = dir.join("occupied");
    fs::create_dir_all(&target_is_dir).unwrap();
    assert!(write_atomic(&target_is_dir, b"data").is_err());
    assert!(!dir.join("occupied.tmp").exists());
    assert!(target_is_dir.is_dir());
    fs::remove_dir_all(&dir).ok();
}
