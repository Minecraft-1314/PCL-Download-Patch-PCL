use pcl_download_patch::{identify, ORIG};

#[test]
fn identify_accepts_pcl_markers() {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"MZ");
    data.extend_from_slice(b"RestrictMcDownload");
    data.extend_from_slice(b"PageDownloadInstall");
    data.extend_from_slice(b"DlClientListLoader");
    data.extend_from_slice(b"MeloongCore");
    data.extend_from_slice(&ORIG);
    let id = identify(&data);
    assert!(id.is_pcl);
    assert!(id.score >= pcl_download_patch::PCL_IDENTIFY_THRESHOLD);
}

#[test]
fn identify_accepts_pcl_without_signature_block() {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"MZ");
    data.extend_from_slice(b"RestrictMcDownload");
    data.extend_from_slice(b"PageDownloadInstall");
    data.extend_from_slice(b"DlClientListLoader");
    data.extend_from_slice(b"MeloongCore");
    assert!(identify(&data).is_pcl);
}

#[test]
fn identify_rejects_unrelated_binary() {
    let data = vec![0x41u8; 8192];
    assert!(!identify(&data).is_pcl);
}

#[test]
fn identify_needs_multiple_signals() {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"MZ");
    data.extend_from_slice(b"MeloongCore");
    assert!(!identify(&data).is_pcl);
}

#[test]
fn identify_needs_threshold_even_with_one_strong_signal() {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"RestrictMcDownload");
    let id = identify(&data);
    assert_eq!(id.score, 45);
    assert!(!id.is_pcl);
}
