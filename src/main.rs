use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use pcl_download_patch::{
    apply_patch, backup_path, ensure_backup, identify, load_lang, load_recent, messages,
    restore, save_lang, save_recent, state_of, state_message, system_lang, write_atomic,
    Identification, Lang, Messages, PatchState,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const ILPATCH_APPHOST: &[u8] = include_bytes!("embedded/ilpatch.exe");
const ILPATCH_ASSEMBLY: &[u8] = include_bytes!("embedded/ilpatch.dll");
const ILPATCH_DEPS: &[u8] = include_bytes!("embedded/ilpatch.deps.json");
const ILPATCH_RUNTIMECONFIG: &[u8] = include_bytes!("embedded/ilpatch.runtimeconfig.json");
const DNLIB_ASSEMBLY: &[u8] = include_bytes!("embedded/dnlib.dll");

const HELP_ZH: &str = "\
PCL Download Patch - 恢复 PCL2 被移除的「下载原版游戏」功能

用法:
  PCLPatch.exe patch   [PCL 路径]   打补丁（默认命令）
  PCLPatch.exe restore [PCL 路径]   还原为原版限制状态
  PCLPatch.exe status  [PCL 路径]   查看当前状态
  PCLPatch.exe help                  显示本帮助

说明:
  - 不带参数直接运行（双击）会打开交互式菜单，支持 ↑↓ 或数字选择
  - 把 PCL 主程序拖到 PCLPatch.exe 上，同样会打开菜单并自动选中该文件
  - 识别 PCL2 基于内容特征（.NET 元数据、专有常量、补丁特征块），不依赖文件名
  - 省略路径时自动在当前目录按内容识别 PCL 主程序
  - 首次修改前自动生成 <原文件名>.bak 备份，已存在时不覆盖
  - 语言: --lang zh 或 --lang en（省略时使用已保存语言或系统语言）
  - 请在 PCL 完全退出后执行
  - 退出码: 0 成功, 1 失败
";

const HELP_EN: &str = "\
PCL Download Patch - Restore the vanilla Minecraft download page removed from PCL2

Usage:
  PCLPatch.exe patch   [PCL path]   Apply patch (default command)
  PCLPatch.exe restore [PCL path]   Restore the original restricted state
  PCLPatch.exe status  [PCL path]   Show current state
  PCLPatch.exe help                  Show this help

Notes:
  - Running without arguments (double-click) opens an interactive menu with arrow/digit selection
  - Dropping a PCL executable onto PCLPatch.exe also opens the menu with that file selected
  - PCL2 is identified by content features (.NET metadata, proprietary constants, signature blocks), not by file name
  - With no path given, the PCL executable is found in the current directory by content
  - A <file>.bak backup is created before the first modification and never overwritten
  - Language: --lang zh or --lang en (falls back to the saved or system language)
  - Close PCL before running
  - Exit codes: 0 success, 1 failure
";

#[cfg(windows)]
fn set_console_utf8() {
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleOutputCP(65001);
    }
}

#[cfg(not(windows))]
fn set_console_utf8() {}

#[cfg(windows)]
fn apply_window_icon() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        LoadImageW, SendMessageW, IMAGE_ICON, LR_DEFAULTCOLOR, WM_SETICON,
    };
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() {
            return;
        }
        let module = GetModuleHandleW(std::ptr::null());
        if module.is_null() {
            return;
        }
        let name = 1 as *const u16;
        let big = LoadImageW(module, name, IMAGE_ICON, 32, 32, LR_DEFAULTCOLOR);
        let small = LoadImageW(module, name, IMAGE_ICON, 16, 16, LR_DEFAULTCOLOR);
        if !big.is_null() {
            SendMessageW(hwnd, WM_SETICON, 1, big as isize);
        }
        if !small.is_null() {
            SendMessageW(hwnd, WM_SETICON, 0, small as isize);
        }
    }
}

#[cfg(not(windows))]
fn apply_window_icon() {}

const COLOR_TITLE: u16 = 0x0B;
const COLOR_OK: u16 = 0x0A;
const COLOR_WARN: u16 = 0x0E;
const COLOR_ERROR: u16 = 0x0C;
const COLOR_SELECTED: u16 = 0x1F;
const COLOR_DIM: u16 = 0x08;

fn state_color(state: PatchState) -> u16 {
    match state {
        PatchState::Patched => COLOR_OK,
        PatchState::Partial => COLOR_WARN,
        PatchState::Restricted => COLOR_WARN,
        PatchState::Unknown => COLOR_ERROR,
    }
}

fn with_color(attr: u16, print_fn: impl FnOnce()) {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
        );
        let mut info: windows_sys::Win32::System::Console::CONSOLE_SCREEN_BUFFER_INFO =
            std::mem::zeroed();
        if windows_sys::Win32::System::Console::GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
            print_fn();
            return;
        }
        if windows_sys::Win32::System::Console::SetConsoleTextAttribute(handle, attr) == 0 {
            print_fn();
            return;
        }
        print_fn();
        windows_sys::Win32::System::Console::SetConsoleTextAttribute(handle, info.wAttributes);
    }
}

fn clear_screen() -> bool {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
        );
        let mut info: windows_sys::Win32::System::Console::CONSOLE_SCREEN_BUFFER_INFO =
            std::mem::zeroed();
        if windows_sys::Win32::System::Console::GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
            return false;
        }
        let count = info.dwSize.X as u32 * info.dwSize.Y as u32;
        let origin = windows_sys::Win32::System::Console::COORD { X: 0, Y: 0 };
        let mut written: u32 = 0;
        if windows_sys::Win32::System::Console::FillConsoleOutputCharacterW(
            handle, 32, count, origin, &mut written,
        ) == 0
        {
            return false;
        }
        windows_sys::Win32::System::Console::SetConsoleCursorPosition(handle, origin) != 0
    }
}

struct RawMode {
    handle: windows_sys::Win32::Foundation::HANDLE,
    original: u32,
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Console::SetConsoleMode(self.handle, self.original);
        }
    }
}

fn enable_raw_mode() -> Option<RawMode> {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
        );
        let mut original: u32 = 0;
        if windows_sys::Win32::System::Console::GetConsoleMode(handle, &mut original) == 0 {
            return None;
        }
        let raw = original
            & !(windows_sys::Win32::System::Console::ENABLE_LINE_INPUT
                | windows_sys::Win32::System::Console::ENABLE_ECHO_INPUT
                | windows_sys::Win32::System::Console::ENABLE_PROCESSED_INPUT)
            | windows_sys::Win32::System::Console::ENABLE_WINDOW_INPUT;
        if windows_sys::Win32::System::Console::SetConsoleMode(handle, raw) == 0 {
            return None;
        }
        Some(RawMode { handle, original })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuKey {
    Up,
    Down,
    Enter,
    Esc,
    Invalid,
    Digit(usize),
}

fn read_key_raw() -> Option<MenuKey> {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
        );
        let mut buffer: [windows_sys::Win32::System::Console::INPUT_RECORD; 8] =
            std::mem::zeroed();
        let mut read: u32 = 0;
        loop {
            if windows_sys::Win32::System::Console::ReadConsoleInputW(
                handle,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut read,
            ) == 0
            {
                return None;
            }
            for record in &buffer[..read as usize] {
                if record.EventType != windows_sys::Win32::System::Console::KEY_EVENT as u16 {
                    continue;
                }
                let key = &record.Event.KeyEvent;
                if key.bKeyDown == 0 {
                    continue;
                }
                match key.wVirtualKeyCode {
                    0x26 => return Some(MenuKey::Up),
                    0x28 => return Some(MenuKey::Down),
                    0x0D => return Some(MenuKey::Enter),
                    0x1B => return Some(MenuKey::Esc),
                    0x43
                        if key.dwControlKeyState
                            & (windows_sys::Win32::System::Console::LEFT_CTRL_PRESSED
                                | windows_sys::Win32::System::Console::RIGHT_CTRL_PRESSED)
                            != 0 =>
                    {
                        return Some(MenuKey::Esc)
                    }
                    vk if (0x30..=0x35).contains(&vk) => {
                        return Some(MenuKey::Digit((vk - 0x30) as usize))
                    }
                    _ => {
                        let ch = key.uChar.UnicodeChar;
                        if (0x30..=0x35).contains(&ch) {
                            return Some(MenuKey::Digit((ch - 0x30) as usize));
                        }
                    }
                }
            }
        }
    }
}

fn read_line() -> Option<String> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}

fn flush_console_input() {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
        );
        windows_sys::Win32::System::Console::FlushConsoleInputBuffer(handle);
    }
}

fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn prompt_lang(dir: &Path) -> Lang {
    println!("选择语言 / Select language:");
    println!("  1) 中文");
    println!("  2) English");
    print!("> ");
    let _ = io::stdout().flush();
    let mut attempts = 0;
    loop {
        match read_line() {
            None => return system_lang(),
            Some(choice) => match choice.as_str() {
                "1" => {
                    let lang = Lang::Zh;
                    save_lang(dir, lang);
                    return lang;
                }
                "2" => {
                    let lang = Lang::En;
                    save_lang(dir, lang);
                    return lang;
                }
                _ => {
                    attempts += 1;
                    if attempts >= 3 {
                        return system_lang();
                    }
                    print!("1 / 2 > ");
                    let _ = io::stdout().flush();
                }
            },
        }
    }
}

fn validate_target(path: &Path) -> Option<Identification> {
    let data = fs::read(path).ok()?;
    let id = identify(&data);
    if id.is_pcl {
        Some(id)
    } else {
        None
    }
}

fn resolve_target(arg: Option<&String>, m: &Messages, silent: bool) -> Option<PathBuf> {
    if let Some(raw) = arg {
        let path = PathBuf::from(raw);
        if !path.is_file() {
            if !silent {
                println!("{}", m.file_not_exist.replace("{}", &path.display().to_string()));
            }
            return None;
        }
        return Some(path);
    }
    if let Some(recent) = load_recent(&exe_dir()) {
        return Some(recent);
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let self_path = env::current_exe().ok();
    let mut best: Option<(PathBuf, u32)> = None;
    let mut tie = false;
    if let Ok(entries) = fs::read_dir(&cwd) {
        for entry in entries.flatten() {
            let path = entry.path();
            if Some(&path) == self_path.as_ref() {
                continue;
            }
            let is_exe = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);
            if !is_exe {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.len() < 1_000_000 {
                continue;
            }
            let Ok(data) = fs::read(&path) else {
                continue;
            };
            let id = identify(&data);
            if !id.is_pcl {
                continue;
            }
            match &best {
                None => best = Some((path, id.score)),
                Some((_, best_score)) if id.score > *best_score => {
                    best = Some((path, id.score));
                    tie = false;
                }
                Some((_, best_score)) if id.score == *best_score => tie = true,
                _ => {}
            }
        }
    }
    if tie {
        if !silent {
            println!("{}", m.multi_pcl);
        }
        return None;
    }
    if best.is_none() && !silent {
        println!("{}", m.no_pcl);
    }
    best.map(|(path, _)| path)
}

fn execute_status(target: &Path, lang: Lang) -> i32 {
    let m = messages(lang);
    let data = match fs::read(target) {
        Ok(data) => data,
        Err(e) => {
            println!("{}", m.read_fail.replace("{}", &format!("{} ({})", target.display(), e)));
            return 1;
        }
    };
    let id = identify(&data);
    if !id.is_pcl {
        println!("{}", m.err_not_pcl);
        return 1;
    }
    println!("{}", m.target.replace("{}", &target.display().to_string()));
    with_color(COLOR_OK, || {
        println!("{}", m.identify_ok.replace("{}", &id.reasons.len().to_string()));
    });
    if let Some(v) = id.version {
        println!("{}", m.identify_version.replace("{}", v));
    }
    let state = state_of(&data);
    with_color(state_color(state), || {
        println!("{}", m.state.replace("{}", state_message(state, lang)));
    });
    if state == PatchState::Unknown {
        return 1;
    }
    0
}

fn execute_patch(target: &Path, lang: Lang) -> i32 {
    let m = messages(lang);
    let data = match fs::read(target) {
        Ok(data) => data,
        Err(e) => {
            println!("{}", m.read_fail.replace("{}", &format!("{} ({})", target.display(), e)));
            return 1;
        }
    };
    let id = identify(&data);
    if !id.is_pcl {
        println!("{}", m.err_not_pcl);
        return 1;
    }
    println!("{}", m.target.replace("{}", &target.display().to_string()));
    if let Some(v) = id.version {
        println!("{}", m.identify_version.replace("{}", v));
    }
    let state = state_of(&data);
    with_color(state_color(state), || {
        println!("{}", m.state.replace("{}", state_message(state, lang)));
    });
    if state == PatchState::Patched {
        println!("{}", m.skip_patched);
        run_ilpatch_if_present(target, &m);
        return 0;
    }
    let mut staged = data.clone();
    if let Err(e) = apply_patch(&mut staged) {
        println!("{} {}", m.fail_tag, e.message(lang));
        return 1;
    }
    match ensure_backup(target) {
        Ok(Some(bak)) => println!("{}", m.backup_new.replace("{}", &bak.display().to_string())),
        Ok(None) => println!("{}", m.backup_keep),
        Err(e) => {
            println!("{} {}", m.backup_fail, e.message(lang));
            return 1;
        }
    }
    if let Err(e) = write_atomic(target, &staged) {
        println!("{}", m.write_fail.replace("{}", &format!("{} ({})", target.display(), e)));
        return 1;
    }
    let reread = fs::read(target).unwrap_or_default();
    if state_of(&reread) != PatchState::Patched {
        println!("{}", m.verify_patch);
        return 1;
    }
    with_color(COLOR_OK, || println!("{}", m.patch_done));
    run_ilpatch_if_present(target, &m);
    0
}

fn run_ilpatch_if_present(target: &Path, m: &Messages) {
    let staging = match stage_ilpatch() {
        Ok(d) => d,
        Err(e) => {
            with_color(COLOR_ERROR, || {
                println!("{} ilpatch stage: {}", m.fail_tag, e);
            });
            return;
        }
    };
    let temp_out = target.with_extension("exe.ilpatch_tmp");
    let run = std::process::Command::new(staging.join("ilpatch.exe"))
        .arg(target)
        .arg(&temp_out)
        .output();
    match run {
        Ok(out) if out.status.success() => {
            if let Err(e) = std::fs::rename(&temp_out, target) {
                with_color(COLOR_ERROR, || {
                    println!("{} ilpatch rename: {}", m.fail_tag, e);
                });
            } else {
                with_color(COLOR_OK, || println!("{}", m.ilpatch_done));
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let detail = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            with_color(COLOR_ERROR, || {
                println!("{} {}", m.fail_tag, detail);
            });
        }
        Err(e) => {
            println!("{} ilpatch: {}", m.fail_tag, e);
        }
    }
    let _ = std::fs::remove_file(&temp_out);
    let _ = std::fs::remove_dir_all(&staging);
}

fn stage_ilpatch() -> Result<PathBuf, String> {
    let dir = env::temp_dir().join(format!("PclPatch-ilpatch-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let files: [(&str, &[u8]); 5] = [
        ("ilpatch.exe", ILPATCH_APPHOST),
        ("ilpatch.dll", ILPATCH_ASSEMBLY),
        ("ilpatch.deps.json", ILPATCH_DEPS),
        ("ilpatch.runtimeconfig.json", ILPATCH_RUNTIMECONFIG),
        ("dnlib.dll", DNLIB_ASSEMBLY),
    ];
    for (name, data) in files {
        fs::write(dir.join(name), data).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

fn execute_restore(target: &Path, lang: Lang) -> i32 {
    let m = messages(lang);
    let data = match fs::read(target) {
        Ok(data) => data,
        Err(e) => {
            println!("{}", m.read_fail.replace("{}", &format!("{} ({})", target.display(), e)));
            return 1;
        }
    };
    let id = identify(&data);
    if !id.is_pcl {
        println!("{}", m.err_not_pcl);
        return 1;
    }
    println!("{}", m.target.replace("{}", &target.display().to_string()));
    if let Some(v) = id.version {
        println!("{}", m.identify_version.replace("{}", v));
    }
    let state = state_of(&data);
    with_color(state_color(state), || {
        println!("{}", m.state.replace("{}", state_message(state, lang)));
    });
    if state == PatchState::Restricted {
        println!("{}", m.skip_restricted);
        return 0;
    }
    let mut staged = data.clone();
    if let Err(e) = restore(&mut staged) {
        println!("{} {}", m.fail_tag, e.message(lang));
        return 1;
    }
    if let Err(e) = write_atomic(target, &staged) {
        println!("{}", m.write_fail.replace("{}", &format!("{} ({})", target.display(), e)));
        return 1;
    }
    let reread = fs::read(target).unwrap_or_default();
    if state_of(&reread) != PatchState::Restricted {
        println!("{}", m.verify_restore);
        return 1;
    }
    with_color(COLOR_OK, || println!("{}", m.restore_done));
    0
}

fn run_command(args: &[String], lang: Lang) -> i32 {
    let command = args.first().map(|s| s.as_str()).unwrap_or("patch");
    let (command, path_arg) = match command {
        "patch" | "restore" | "status" => (command, args.get(1)),
        _ => ("patch", args.first()),
    };
    let target = match resolve_target(path_arg, messages(lang), false) {
        Some(target) => target,
        None => return 1,
    };
    if path_arg.is_some() {
        save_recent(&exe_dir(), &target);
    }
    match command {
        "patch" => execute_patch(&target, lang),
        "restore" => execute_restore(&target, lang),
        "status" => execute_status(&target, lang),
        _ => unreachable!(),
    }
}

const ITEM_COUNT: usize = 6;

fn render_menu(
    lang: Lang,
    chosen: Option<&(PathBuf, Identification)>,
    state: Option<PatchState>,
    selected: usize,
) {
    let m = messages(lang);
    let _ = clear_screen();
    with_color(COLOR_TITLE, || println!("PCL Download Patch v{}", VERSION));
    match chosen {
        Some((target, id)) => {
            println!("{}", m.target.replace("{}", &target.display().to_string()));
            with_color(COLOR_OK, || {
                println!("{}", m.identify_ok.replace("{}", &id.reasons.len().to_string()));
            });
            if let Some(v) = id.version {
                println!("{}", m.identify_version.replace("{}", v));
            }
            if let Some(state) = state {
                with_color(state_color(state), || {
                    println!("{}", m.state.replace("{}", state_message(state, lang)));
                });
                let backup = backup_path(target);
                let backup_text = if backup.exists() { m.backup_yes } else { m.backup_no };
                println!("{}", m.backup_state.replace("{}", backup_text));
                with_color(
                    if state == PatchState::Patched { COLOR_OK } else { COLOR_WARN },
                    || {
                        let hint = match state {
                            PatchState::Restricted => m.hint_restricted,
                            PatchState::Partial => m.hint_partial,
                            PatchState::Patched => m.hint_patched,
                            PatchState::Unknown => m.hint_pcl_mismatch,
                        };
                        println!("{}", hint);
                    },
                );
            }
        }
        None => {
            with_color(COLOR_WARN, || println!("{}", m.no_target_menu));
        }
    }
    println!();
    let items: [String; ITEM_COUNT] = [
        m.menu_patch.to_string(),
        m.menu_restore.to_string(),
        m.menu_status.to_string(),
        m.menu_lang_cur.replace("{}", lang.label()),
        m.menu_path.to_string(),
        m.menu_exit.to_string(),
    ];
    for (index, text) in items.iter().enumerate() {
        if index == selected {
            with_color(COLOR_SELECTED, || println!("  > {}", text));
        } else {
            println!("    {}", text);
        }
    }
    println!();
    with_color(COLOR_DIM, || {
        println!("{}", if raw_available() { m.controls_raw } else { m.controls_line });
    });
}

fn raw_available() -> bool {
    unsafe {
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
        );
        let mut mode: u32 = 0;
        windows_sys::Win32::System::Console::GetConsoleMode(handle, &mut mode) != 0
    }
}

fn read_menu_key(raw: &mut Option<RawMode>) -> Option<MenuKey> {
    if raw.is_some() {
        read_key_raw()
    } else {
        let line = read_line()?;
        match line.as_str() {
            "0" => Some(MenuKey::Digit(0)),
            "1" => Some(MenuKey::Digit(1)),
            "2" => Some(MenuKey::Digit(2)),
            "3" => Some(MenuKey::Digit(3)),
            "4" => Some(MenuKey::Digit(4)),
            "5" => Some(MenuKey::Digit(5)),
            "" => Some(MenuKey::Invalid),
            _ => Some(MenuKey::Invalid),
        }
    }
}

fn pause_menu(raw: &mut Option<RawMode>, m: &Messages) -> bool {
    println!();
    println!("{}", m.press_enter);
    if raw.is_some() {
        read_key_raw().is_some()
    } else {
        read_line().is_some()
    }
}

fn run_menu(mut lang: Lang, preset: Option<PathBuf>) {
    let dir = exe_dir();
    let mut chosen: Option<(PathBuf, Identification)> = None;
    let mut selected: usize = 0;
    let mut raw = enable_raw_mode();

    if let Some(path) = preset {
        match validate_target(&path) {
            Some(id) => {
                save_recent(&dir, &path);
                chosen = Some((path, id));
            }
            None => {
                println!("{}", messages(lang).err_not_pcl);
                pause_menu(&mut raw, messages(lang));
            }
        }
    }

    loop {
        let m = messages(lang);
        if chosen.is_none() {
            if let Some(path) = resolve_target(None, m, true) {
                if let Some(id) = validate_target(&path) {
                    chosen = Some((path, id));
                }
            }
        }
        let mut state: Option<PatchState> = None;
        if let Some((target, _)) = &chosen {
            state = fs::read(target).ok().map(|data| state_of(&data));
        }
        render_menu(lang, chosen.as_ref(), state, selected);
        print!("{} ", m.select);
        let _ = io::stdout().flush();
        let key = match read_menu_key(&mut raw) {
            Some(key) => key,
            None => break,
        };
        match key {
            MenuKey::Up => {
                selected = (selected + ITEM_COUNT - 1) % ITEM_COUNT;
            }
            MenuKey::Down => {
                selected = (selected + 1) % ITEM_COUNT;
            }
            MenuKey::Esc => break,
            MenuKey::Invalid => {}
            MenuKey::Digit(0) => break,
            MenuKey::Enter | MenuKey::Digit(_) => {
                let index = match key {
                    MenuKey::Digit(number) => number - 1,
                    _ => selected,
                };
                selected = index;
                match index {
                    0 | 1 | 2 => {
                        let code = match &chosen {
                            Some((target, _)) => match index {
                                0 => execute_patch(target, lang),
                                1 => execute_restore(target, lang),
                                _ => execute_status(target, lang),
                            },
                            None => {
                                println!("{}", m.no_target_menu);
                                1
                            }
                        };
                        let _ = code;
                        if !pause_menu(&mut raw, m) {
                            break;
                        }
                    }
                    3 => {
                        lang = match lang {
                            Lang::Zh => Lang::En,
                            Lang::En => Lang::Zh,
                        };
                        save_lang(&dir, lang);
                        println!("{}", messages(lang).lang_saved);
                        continue;
                    }
                    4 => {
                        println!("{}", m.input_path);
                        let _ = raw.take();
                        flush_console_input();
                        let line = read_line();
                        raw = enable_raw_mode();
                        match line {
                            None => break,
                            Some(line) => {
                                let cleaned = line.trim().trim_matches('"');
                                if cleaned.is_empty() {
                                    continue;
                                }
                                let path = PathBuf::from(cleaned);
                                if !path.is_file() {
                                    println!("{}", m.file_not_exist.replace("{}", &path.display().to_string()));
                                    if !pause_menu(&mut raw, m) {
                                        break;
                                    }
                                    continue;
                                }
                                match validate_target(&path) {
                                    Some(id) => {
                                        save_recent(&dir, &path);
                                        chosen = Some((path, id));
                                    }
                                    None => {
                                        println!("{}", m.err_not_pcl);
                                        if !pause_menu(&mut raw, m) {
                                            break;
                                        }
                                    }
                                }
                                continue;
                            }
                        }
                    }
                    _ => break,
                }
            }
        }
    }
}

fn main() {
    set_console_utf8();
    apply_window_icon();
    let args: Vec<String> = env::args().skip(1).collect();

    let mut lang_flag: Option<Lang> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--lang" || args[index] == "-l" {
            if index + 1 < args.len() {
                lang_flag = Lang::parse(&args[index + 1]);
                index += 2;
            } else {
                index += 1;
            }
        } else {
            rest.push(args[index].clone());
            index += 1;
        }
    }

    let is_subcommand = matches!(
        rest.first().map(|s| s.as_str()),
        Some("patch") | Some("restore") | Some("status") | Some("help") | Some("-h") | Some("--help")
    );
    let drag_dropped = rest.len() == 1 && !is_subcommand && Path::new(&rest[0]).is_file();
    let interactive = rest.is_empty() || drag_dropped;

    let dir = exe_dir();
    let lang = match lang_flag {
        Some(lang) => {
            save_lang(&dir, lang);
            lang
        }
        None => match load_lang(&dir) {
            Some(lang) => lang,
            None => {
                if interactive {
                    prompt_lang(&dir)
                } else {
                    system_lang()
                }
            }
        },
    };

    if interactive {
        let preset = if drag_dropped {
            Some(PathBuf::from(&rest[0]))
        } else {
            None
        };
        run_menu(lang, preset);
        return;
    }

    if rest[0] == "help" || rest[0] == "-h" || rest[0] == "--help" {
        println!("{}", if lang == Lang::Zh { HELP_ZH } else { HELP_EN });
        return;
    }

    exit(run_command(&rest, lang));
}
