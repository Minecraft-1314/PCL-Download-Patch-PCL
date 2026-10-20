use pcl_download_patch::{
    apply_patch, count, locate, restore, state_of, PatchError, PatchState, NEW, ORIG, TAIL,
};

fn fixture_orig() -> Vec<u8> {
    let mut data = vec![0xAAu8; 4096];
    data[512..512 + ORIG.len()].copy_from_slice(&ORIG);
    data[512 + ORIG.len()..512 + ORIG.len() + TAIL.len()].copy_from_slice(&TAIL);
    data
}

fn fixture_patched() -> Vec<u8> {
    let mut data = fixture_orig();
    apply_patch(&mut data).unwrap();
    data
}

#[test]
fn locate_finds_original_block() {
    assert_eq!(locate(&fixture_orig()), Some(512));
}

#[test]
fn original_state_is_restricted() {
    assert_eq!(state_of(&fixture_orig()), PatchState::Restricted);
}

#[test]
fn apply_produces_patched_state() {
    let mut data = fixture_orig();
    apply_patch(&mut data).unwrap();
    assert_eq!(state_of(&data), PatchState::Patched);
    assert_eq!(data[512..512 + NEW.len()], NEW);
}

#[test]
fn apply_twice_reports_already_patched() {
    let mut data = fixture_patched();
    assert!(matches!(apply_patch(&mut data), Err(PatchError::AlreadyPatched)));
}

#[test]
fn restore_returns_to_original_bytes() {
    let mut data = fixture_patched();
    restore(&mut data).unwrap();
    assert_eq!(state_of(&data), PatchState::Restricted);
    assert_eq!(data[512..512 + ORIG.len()], ORIG);
}

#[test]
fn restore_on_original_reports_already_original() {
    let mut data = fixture_orig();
    assert!(matches!(restore(&mut data), Err(PatchError::AlreadyOriginal)));
}

#[test]
fn damaged_block_is_refused_for_writes() {
    let mut data = fixture_orig();
    data[512..517].copy_from_slice(&[0x16, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(state_of(&data), PatchState::Unknown);
    assert!(matches!(apply_patch(&mut data), Err(PatchError::PatternDamaged)));
    assert!(matches!(restore(&mut data), Err(PatchError::PatternDamaged)));
    assert_eq!(data[512..517], [0x16, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn missing_marker_is_rejected() {
    let mut data = vec![0x00u8; 4096];
    assert!(matches!(apply_patch(&mut data), Err(PatchError::PatternNotFound)));
    assert!(matches!(restore(&mut data), Err(PatchError::PatternNotFound)));
    assert_eq!(count(&data, &ORIG), 0);
}

#[test]
fn duplicated_marker_is_rejected() {
    let mut data = fixture_orig();
    data.extend_from_slice(&ORIG);
    assert!(matches!(apply_patch(&mut data), Err(PatchError::AmbiguousPattern)));
    assert!(matches!(restore(&mut data), Err(PatchError::AmbiguousPattern)));
}

#[test]
fn tail_anchor_too_early_is_ignored() {
    let mut data = vec![0x00u8; 64];
    data[0..TAIL.len()].copy_from_slice(&TAIL);
    assert_eq!(locate(&data), None);
}

#[test]
fn patch_preserves_file_length() {
    let mut data = fixture_orig();
    let before = data.len();
    apply_patch(&mut data).unwrap();
    assert_eq!(data.len(), before);
}
