use std::fs;
use std::path::{Path, PathBuf};

pub const ORIG: [u8; 48] = [
    0x72, 0x33, 0x47, 0x01, 0x70,
    0x7e, 0xfb, 0x09, 0x00, 0x04,
    0x2c, 0x07,
    0x7e, 0xfb, 0x09, 0x00, 0x04,
    0x2b, 0x16,
    0x7e, 0xf9, 0x09, 0x00, 0x04,
    0xfe, 0x06, 0x67, 0x14, 0x00, 0x06,
    0x73, 0x8d, 0x05, 0x00, 0x0a,
    0x25,
    0x80, 0xfb, 0x09, 0x00, 0x04,
    0x14,
    0x18,
    0x73, 0x8e, 0x05, 0x00, 0x0a,
];

pub const NEW: [u8; 48] = [
    0x7e, 0x0f, 0x04, 0x00, 0x04,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00,
];

pub const TAIL: [u8; 20] = [
    0x02, 0xfe, 0x06, 0x1c, 0x06, 0x00, 0x06,
    0x73, 0x8d, 0x05, 0x00, 0x0a,
    0x14,
    0x17,
    0x28, 0xca, 0x00, 0x00,
    0x2b,
    0x2a,
];

pub const DEFAULT_EXE: &str = "Plain Craft Launcher 2.exe";
pub const LANG_FILE_NAME: &str = "PCLPatch.lang";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchState {
    Restricted,
    Patched,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn parse(text: &str) -> Option<Lang> {
        match text.trim().to_ascii_lowercase().as_str() {
            "zh" | "cn" | "chinese" | "中文" => Some(Lang::Zh),
            "en" | "english" | "英文" => Some(Lang::En),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::En => "English",
        }
    }
}

#[cfg(windows)]
pub fn system_lang() -> Lang {
    unsafe {
        let id = windows_sys::Win32::Globalization::GetUserDefaultUILanguage();
        if id & 0x03FF == 0x0004 {
            Lang::Zh
        } else {
            Lang::En
        }
    }
}

#[cfg(not(windows))]
pub fn system_lang() -> Lang {
    Lang::Zh
}

pub fn lang_file_path(dir: &Path) -> PathBuf {
    dir.join(LANG_FILE_NAME)
}

pub fn load_lang(dir: &Path) -> Option<Lang> {
    let text = fs::read_to_string(lang_file_path(dir)).ok()?;
    Lang::parse(&text)
}

pub fn save_lang(dir: &Path, lang: Lang) -> bool {
    let text = match lang {
        Lang::Zh => "zh",
        Lang::En => "en",
    };
    fs::write(lang_file_path(dir), text).is_ok()
}

pub struct Messages {
    pub fail_tag: &'static str,
    pub target: &'static str,
    pub state: &'static str,
    pub state_patched: &'static str,
    pub state_restricted: &'static str,
    pub state_unknown: &'static str,
    pub patch_done: &'static str,
    pub restore_done: &'static str,
    pub skip_patched: &'static str,
    pub skip_restricted: &'static str,
    pub backup_new: &'static str,
    pub backup_keep: &'static str,
    pub backup_fail: &'static str,
    pub verify_patch: &'static str,
    pub verify_restore: &'static str,
    pub read_fail: &'static str,
    pub write_fail: &'static str,
    pub file_not_exist: &'static str,
    pub no_pcl: &'static str,
    pub multi_pcl: &'static str,
    pub err_io: &'static str,
    pub err_not_found: &'static str,
    pub err_ambiguous: &'static str,
    pub err_already_patched: &'static str,
    pub err_already_original: &'static str,
    pub err_verify: &'static str,
    pub menu_patch: &'static str,
    pub menu_restore: &'static str,
    pub menu_status: &'static str,
    pub menu_lang_cur: &'static str,
    pub menu_path: &'static str,
    pub menu_exit: &'static str,
    pub select: &'static str,
    pub no_target_menu: &'static str,
    pub press_enter: &'static str,
    pub input_path: &'static str,
    pub lang_saved: &'static str,
    pub backup_state: &'static str,
    pub backup_yes: &'static str,
    pub backup_no: &'static str,
    pub hint_restricted: &'static str,
    pub hint_patched: &'static str,
    pub hint_unknown: &'static str,
    pub controls_raw: &'static str,
    pub controls_line: &'static str,
    pub identify_ok: &'static str,
    pub err_not_pcl: &'static str,
    pub err_damaged: &'static str,
    pub hint_pcl_mismatch: &'static str,
}

pub const ZH: Messages = Messages {
    fail_tag: "[失败]",
    target: "目标文件: {}",
    state: "当前状态: {}",
    state_patched: "已打补丁（下载功能已恢复）",
    state_restricted: "受限制（原版行为）",
    state_unknown: "无法识别（可能不是适配的 PCL 版本）",
    patch_done: "[完成] 补丁已写入，重启 PCL 后下载原版游戏功能恢复",
    restore_done: "[完成] 已还原为原版限制状态",
    skip_patched: "[跳过] 已经是补丁状态，无需重复操作",
    skip_restricted: "[跳过] 已经是原版状态，无需还原",
    backup_new: "[备份] {}",
    backup_keep: "[备份] 已存在备份，保留不覆盖",
    backup_fail: "创建备份失败: {}",
    verify_patch: "写入后校验失败，可用 restore 还原后重试",
    verify_restore: "写入后校验失败，请重试",
    read_fail: "读取失败 {}",
    write_fail: "写入失败 {}（PCL 是否正在运行？）",
    file_not_exist: "文件不存在: {}",
    no_pcl: "未找到 PCL 主程序：请把 PCLPatch.exe 放进 PCL 目录后运行，或用参数指定路径",
    multi_pcl: "当前目录存在多个 PCL 主程序，请用参数明确指定路径",
    err_io: "文件读写错误: {}",
    err_not_found: "未找到适配的特征块：当前 PCL 版本可能不受支持，或官方已更新该处实现",
    err_ambiguous: "特征块出现多次，为安全起见拒绝修改",
    err_already_patched: "已经是补丁状态",
    err_already_original: "已经是原版状态",
    err_verify: "写入后校验失败",
    menu_patch: "1) 打补丁",
    menu_restore: "2) 还原原版",
    menu_status: "3) 查看状态",
    menu_lang_cur: "4) 切换语言 (当前: {})",
    menu_path: "5) 指定 PCL 路径",
    menu_exit: "0) 退出",
    select: "请选择:",
    no_target_menu: "未找到 PCL 主程序，请先用选项 5 指定路径",
    press_enter: "按回车返回菜单...",
    input_path: "输入 Plain Craft Launcher 2.exe 的完整路径（直接回车返回）:",
    lang_saved: "语言已保存",
    backup_state: "备份文件: {}",
    backup_yes: "已存在",
    backup_no: "无（首次打补丁时自动创建）",
    hint_restricted: "提示: 当前受限制，选择 1) 打补丁 可恢复下载",
    hint_patched: "提示: 下载功能已恢复，无需其他操作",
    hint_unknown: "提示: 无法识别该文件，未做任何修改；如路径不对请用 5) 指定 PCL 路径",
    controls_raw: "↑↓ 或数字选择，Enter 确认，Esc 退出",
    controls_line: "输入数字选择，0 退出",
    identify_ok: "识别: PCL2（匹配特征 {} 项）",
    err_not_pcl: "该文件不是 PCL2 主程序，已取消操作",
    err_damaged: "特征块状态异常（既非原版也非补丁形态），已拒绝修改；请用 .bak 备份还原或重新获取文件",
    hint_pcl_mismatch: "提示: 这是 PCL2，但未找到适配的特征块（版本可能过新或过旧），未做任何修改",
};

pub const EN: Messages = Messages {
    fail_tag: "[Fail]",
    target: "Target file: {}",
    state: "Current state: {}",
    state_patched: "Patched (vanilla download restored)",
    state_restricted: "Restricted (vanilla behavior)",
    state_unknown: "Unrecognized (PCL version may be unsupported)",
    patch_done: "[Done] Patch written. Restart PCL to bring back the vanilla download page",
    restore_done: "[Done] Restored to the original restricted state",
    skip_patched: "[Skip] Already patched, nothing to do",
    skip_restricted: "[Skip] Already in the original state, nothing to restore",
    backup_new: "[Backup] {}",
    backup_keep: "[Backup] Existing backup kept",
    backup_fail: "Failed to create backup: {}",
    verify_patch: "Post-write verification failed; run restore and retry",
    verify_restore: "Post-write verification failed; please retry",
    read_fail: "Failed to read {}",
    write_fail: "Failed to write {} (is PCL running?)",
    file_not_exist: "File not found: {}",
    no_pcl: "PCL executable not found: place PCLPatch.exe in the PCL folder, or pass the path as an argument",
    multi_pcl: "Multiple PCL executables found; specify the path explicitly",
    err_io: "File I/O error: {}",
    err_not_found: "Signature block not found: this PCL version may be unsupported, or the implementation has changed",
    err_ambiguous: "Signature block found multiple times; refusing to modify for safety",
    err_already_patched: "Already patched",
    err_already_original: "Already in the original state",
    err_verify: "Post-write verification failed",
    menu_patch: "1) Apply patch",
    menu_restore: "2) Restore original",
    menu_status: "3) Show status",
    menu_lang_cur: "4) Switch language (current: {})",
    menu_path: "5) Set PCL path",
    menu_exit: "0) Exit",
    select: "Select:",
    no_target_menu: "PCL executable not found; use option 5 to set the path first",
    press_enter: "Press Enter to return to the menu...",
    input_path: "Enter the full path to Plain Craft Launcher 2.exe (Enter to go back):",
    lang_saved: "Language saved",
    backup_state: "Backup file: {}",
    backup_yes: "exists",
    backup_no: "none (created automatically on first patch)",
    hint_restricted: "Tip: currently restricted - choose 1) Apply patch to restore downloads",
    hint_patched: "Tip: downloads are restored; no action needed",
    hint_unknown: "Tip: file unrecognized, nothing modified; if the path is wrong use 5) Set PCL path",
    controls_raw: "Arrows or digits to select, Enter to confirm, Esc to exit",
    controls_line: "Type a number to select, 0 to exit",
    identify_ok: "Identified: PCL2 ({} features matched)",
    err_not_pcl: "This file is not a PCL2 executable; operation cancelled",
    err_damaged: "The signature block is in an unexpected state (neither original nor patched); refusing to modify. Restore from the .bak backup or re-obtain the file",
    hint_pcl_mismatch: "Tip: this is PCL2, but no compatible signature block was found (version may be too new or too old); nothing was modified",
};

pub fn messages(lang: Lang) -> &'static Messages {
    match lang {
        Lang::Zh => &ZH,
        Lang::En => &EN,
    }
}

pub fn state_message(state: PatchState, lang: Lang) -> &'static str {
    let m = messages(lang);
    match state {
        PatchState::Patched => m.state_patched,
        PatchState::Restricted => m.state_restricted,
        PatchState::Unknown => m.state_unknown,
    }
}

#[derive(Debug)]
pub enum PatchError {
    Io(std::io::Error),
    PatternNotFound,
    PatternDamaged,
    AmbiguousPattern,
    AlreadyPatched,
    AlreadyOriginal,
    VerifyFailed,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message(Lang::Zh))
    }
}

impl std::error::Error for PatchError {}

impl From<std::io::Error> for PatchError {
    fn from(e: std::io::Error) -> Self {
        PatchError::Io(e)
    }
}

impl PatchError {
    pub fn message(&self, lang: Lang) -> String {
        let m = messages(lang);
        fn tpl(template: &str, value: &str) -> String {
            template.replace("{}", value)
        }
        match self {
            PatchError::Io(e) => tpl(m.err_io, &e.to_string()),
            PatchError::PatternNotFound => m.err_not_found.to_string(),
            PatchError::PatternDamaged => m.err_damaged.to_string(),
            PatchError::AmbiguousPattern => m.err_ambiguous.to_string(),
            PatchError::AlreadyPatched => m.err_already_patched.to_string(),
            PatchError::AlreadyOriginal => m.err_already_original.to_string(),
            PatchError::VerifyFailed => m.err_verify.to_string(),
        }
    }
}

fn matches(data: &[u8], at: usize, pattern: &[u8]) -> bool {
    at + pattern.len() <= data.len() && &data[at..at + pattern.len()] == pattern
}

pub fn find(data: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || data.len() < needle.len() {
        return None;
    }
    let first = needle[0];
    let mut i = 0;
    while i + needle.len() <= data.len() {
        if data[i] == first && &data[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn count(data: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || data.len() < needle.len() {
        return 0;
    }
    (0..=data.len() - needle.len())
        .filter(|&i| &data[i..i + needle.len()] == needle)
        .count()
}

pub fn locate(data: &[u8]) -> Option<usize> {
    if let Some(p) = find(data, &ORIG) {
        return Some(p);
    }
    let t = find(data, &TAIL)?;
    if t >= ORIG.len() {
        Some(t - ORIG.len())
    } else {
        None
    }
}

pub fn state_of(data: &[u8]) -> PatchState {
    match locate(data) {
        Some(p) if matches(data, p, &NEW) => PatchState::Patched,
        Some(p) if matches(data, p, &ORIG) => PatchState::Restricted,
        _ => PatchState::Unknown,
    }
}

pub const PCL_IDENTIFY_THRESHOLD: u32 = 50;

#[derive(Debug, Clone)]
pub struct Identification {
    pub score: u32,
    pub is_pcl: bool,
    pub reasons: Vec<&'static str>,
}

fn find_utf16le(data: &[u8], text: &str) -> bool {
    let pattern: Vec<u8> = text
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    find(data, &pattern).is_some()
}

pub fn identify(data: &[u8]) -> Identification {
    let mut score: u32 = 0;
    let mut reasons: Vec<&'static str> = Vec::new();
    let mut add = |points: u32, reason: &'static str| {
        score += points;
        reasons.push(reason);
    };
    if data.starts_with(b"MZ") {
        add(5, "PE 可执行文件");
    }
    if find(data, b"RestrictMcDownload").is_some() {
        add(45, "含 PCL 专有常量 RestrictMcDownload");
    }
    if find(data, b"PageDownloadInstall").is_some() {
        add(15, "含 PCL 下载页类型 PageDownloadInstall");
    }
    if find(data, b"DlClientListLoader").is_some() {
        add(15, "含 PCL 版本列表加载器 DlClientListLoader");
    }
    if find(data, b"MeloongCore").is_some() {
        add(10, "引用 PCL 核心库 MeloongCore");
    }
    if find_utf16le(data, "Plain Craft Launcher") {
        add(5, "版本信息含 Plain Craft Launcher");
    }
    if find_utf16le(data, "为遵守相关法律法规") {
        add(5, "含 PCL 限制提示文案");
    }
    if state_of(data) != PatchState::Unknown {
        add(25, "含补丁特征块");
    }
    Identification {
        score,
        is_pcl: score >= PCL_IDENTIFY_THRESHOLD,
        reasons,
    }
}

fn check_anchors(data: &[u8]) -> Result<(), PatchError> {
    if count(data, &ORIG) > 1 || count(data, &NEW) > 1 || count(data, &TAIL) > 1 {
        return Err(PatchError::AmbiguousPattern);
    }
    Ok(())
}

pub fn apply_patch(data: &mut [u8]) -> Result<(), PatchError> {
    check_anchors(data)?;
    let p = locate(data).ok_or(PatchError::PatternNotFound)?;
    if matches(data, p, &NEW) {
        return Err(PatchError::AlreadyPatched);
    }
    if !matches(data, p, &ORIG) {
        return Err(PatchError::PatternDamaged);
    }
    data[p..p + NEW.len()].copy_from_slice(&NEW);
    if state_of(data) != PatchState::Patched {
        return Err(PatchError::VerifyFailed);
    }
    Ok(())
}

pub fn restore(data: &mut [u8]) -> Result<(), PatchError> {
    check_anchors(data)?;
    let p = locate(data).ok_or(PatchError::PatternNotFound)?;
    if matches(data, p, &ORIG) {
        return Err(PatchError::AlreadyOriginal);
    }
    if !matches(data, p, &NEW) {
        return Err(PatchError::PatternDamaged);
    }
    data[p..p + ORIG.len()].copy_from_slice(&ORIG);
    if state_of(data) != PatchState::Restricted {
        return Err(PatchError::VerifyFailed);
    }
    Ok(())
}

pub fn backup_path(exe: &Path) -> PathBuf {
    let mut s = exe.as_os_str().to_os_string();
    s.push(".bak");
    PathBuf::from(s)
}

pub fn ensure_backup(exe: &Path) -> Result<Option<PathBuf>, PatchError> {
    let bak = backup_path(exe);
    if bak.exists() {
        return Ok(None);
    }
    fs::copy(exe, &bak)?;
    Ok(Some(bak))
}

pub fn write_atomic(target: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let mut tmp_name = target.as_os_str().to_os_string();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    let result = (|| -> Result<(), std::io::Error> {
        fs::write(&tmp, data)?;
        fs::rename(&tmp, target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
