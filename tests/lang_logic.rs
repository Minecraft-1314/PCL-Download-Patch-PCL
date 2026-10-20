use pcl_download_patch::{load_lang, save_lang, Lang, LANG_FILE_NAME};

#[test]
fn parse_languages() {
    assert_eq!(Lang::parse("zh"), Some(Lang::Zh));
    assert_eq!(Lang::parse("EN"), Some(Lang::En));
    assert_eq!(Lang::parse("中文"), Some(Lang::Zh));
    assert_eq!(Lang::parse("英文"), Some(Lang::En));
    assert_eq!(Lang::parse("junk"), None);
    assert_eq!(Lang::parse("  zh  "), Some(Lang::Zh));
}

#[test]
fn lang_file_roundtrip() {
    let dir = std::env::temp_dir().join(format!("pclpatch-lang-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(load_lang(&dir), None);
    assert!(save_lang(&dir, Lang::En));
    assert_eq!(load_lang(&dir), Some(Lang::En));
    assert!(save_lang(&dir, Lang::Zh));
    assert_eq!(load_lang(&dir), Some(Lang::Zh));
    assert!(dir.join(LANG_FILE_NAME).exists());
    std::fs::remove_dir_all(&dir).ok();
}
