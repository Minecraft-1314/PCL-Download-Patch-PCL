
use crate::meta::CliImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointState {
    Pending,
    Done,
    DoneLegacy,
    NotApplicable,
    Changed,
}

impl PointState {
    pub fn is_done(self) -> bool {
        matches!(self, PointState::Done | PointState::DoneLegacy)
    }
}

pub struct PointResult {
    pub name: &'static str,
    pub state: PointState,
    pub code_off: usize,
    pub positions: Vec<usize>,
    pub code_len: usize,
}

pub struct ScanResult {
    pub points: Vec<PointResult>,
    pub v1_set: bool,
}

pub struct Pattern {
    pub b: Vec<u8>,
    pub m: Vec<u8>,
}

fn pat(spec: &str) -> Pattern {
    let mut b = Vec::new();
    let mut m = Vec::new();
    for piece in spec.split_whitespace() {
        if piece == "??" {
            b.push(0);
            m.push(0);
        } else {
            b.push(u8::from_str_radix(piece, 16).expect("bad pattern byte"));
            m.push(1);
        }
    }
    Pattern { b, m }
}

fn matches_at(data: &[u8], p: &Pattern) -> bool {
    if data.len() < p.b.len() {
        return false;
    }
    (0..p.b.len()).all(|i| p.m[i] == 0 || data[i] == p.b[i])
}

fn find_all(code: &[u8], p: &Pattern) -> Vec<usize> {
    let mut out = Vec::new();
    if code.len() < p.b.len() {
        return out;
    }
    for i in 0..=code.len() - p.b.len() {
        if matches_at(&code[i..], p) {
            out.push(i);
        }
    }
    out
}

const NS: &str = "PCL";

struct Located {
    code_off: usize,
    code: Vec<u8>,
}

fn locate_method<'a>(
    img: &'a CliImage,
    ty: &str,
    method: &str,
) -> Option<Located> {
    let (_, rva) = img.find_method(NS, ty, method)?;
    let (code_off, code) = img.method_il(rva)?;
    Some(Located { code_off, code })
}

fn p1_orig() -> Pattern {
    pat("72 ?? ?? ?? 70 7e ?? ?? ?? 04 2c 07 7e ?? ?? ?? 04 2b 16 7e ?? ?? ?? 04 fe 06 ?? ?? ?? 06 73 ?? ?? ?? 0a 25 80 ?? ?? ?? 04 14 18 73 ?? ?? ?? 0a")
}

fn p1_legacy() -> Pattern {
    pat("7e ?? ?? ?? 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00")
}

fn p1_embedded() -> Pattern {
    pat("7e ?? ?? ?? 04 2b 29 ?? ?? ?? 2c 07 7e ?? ?? ?? 04 2b 16 7e ?? ?? ?? 04 fe 06 ?? ?? ?? 06 73 ?? ?? ?? 0a 25 80 ?? ?? ?? 04 14 18 73 ?? ?? ?? 0a")
}

fn p2_orig() -> Pattern {
    pat("02 28 ?? ?? ?? 06 02 16 7d ?? ?? ?? 04 02 28 ?? ?? ?? 06 02 6f ?? ?? ?? 06 18 6f ?? ?? ?? 0a 02 6f ?? ?? ?? 06 18 6f ?? ?? ?? 0a 02 18 7d ?? ?? ?? 04 02 6f ?? ?? ?? 06 17 6f ?? ?? ?? 06 2a")
}

fn p2_new() -> Pattern {
    pat("02 28 ?? ?? ?? 06 02 16 7d ?? ?? ?? 04 02 28 ?? ?? ?? 06 02 6f ?? ?? ?? 06 16 6f ?? ?? ?? 0a 02 6f ?? ?? ?? 06 16 6f ?? ?? ?? 0a 02 16 7d ?? ?? ?? 04 02 6f ?? ?? ?? 06 17 6f ?? ?? ?? 06 2a")
}

fn p3_orig() -> Pattern {
    pat("28 ?? ?? ?? 06 2d 04 17 0a 2b 02 19 0a 06")
}
fn p3_new() -> Pattern {
    pat("28 ?? ?? ?? 06 2d 04 18 0a 2b 02 19 0a 06")
}

fn p4_orig() -> Pattern {
    pat("18 0a 02 6f ?? ?? ?? 06 6f ?? ?? ?? 0a 06 2e")
}
fn p4_new() -> Pattern {
    pat("16 0a 02 6f ?? ?? ?? 06 6f ?? ?? ?? 0a 06 2e")
}

fn p5_orig() -> Pattern {
    pat("02 6f ?? ?? ?? 06 03 18 26 26 18 6f ?? ?? ?? 0a 2a")
}
fn p5_new() -> Pattern {
    pat("02 6f ?? ?? ?? 06 03 18 26 26 16 6f ?? ?? ?? 0a 2a")
}

fn p6_orig() -> Pattern {
    pat("72 ?? ?? ?? 70 6f ?? ?? ?? 0a 02 6f ?? ?? ?? 06 18 6f ?? ?? ?? 0a 2b ??")
}
fn p6_new() -> Pattern {
    pat("72 ?? ?? ?? 70 6f ?? ?? ?? 0a 02 6f ?? ?? ?? 06 16 6f ?? ?? ?? 0a 2b ??")
}

fn p1_ldstr_token(img: &CliImage) -> Option<u32> {
    img.find_us_token(|s| s == "关闭下载")
}

fn p1_dl_field_token(img: &CliImage) -> Option<u32> {
    img.find_field_token(NS, "ModDownload", "DlClientListLoader")
}

fn p2_install_token(img: &CliImage) -> Option<u32> {
    let (rid, _) = img.find_method(NS, "PageDownloadLeft", "get_ItemInstall")?;
    Some(img.method_token(rid))
}

fn p2_mod_token(img: &CliImage) -> Option<u32> {
    let (rid, _) = img.find_method(NS, "PageDownloadLeft", "get_ItemMod")?;
    Some(img.method_token(rid))
}

fn scan_point(
    img: &CliImage,
    name: &'static str,
    ty: &str,
    method: &str,
    orig: &Pattern,
    new: &Pattern,
) -> PointResult {
    let miss = PointResult {
        name,
        state: PointState::NotApplicable,
        code_off: 0,
        positions: Vec::new(),
        code_len: 0,
    };
    let located = match locate_method(img, ty, method) {
        Some(l) => l,
        None => return miss,
    };
    let o = find_all(&located.code, orig);
    let n = find_all(&located.code, new);
    let (state, positions) = if !o.is_empty() {
        (PointState::Pending, o)
    } else if !n.is_empty() {
        (PointState::Done, n)
    } else {
        (PointState::NotApplicable, Vec::new())
    };
    PointResult {
        name,
        state,
        code_off: located.code_off,
        positions,
        code_len: located.code.len(),
    }
}

pub fn scan(img: &CliImage) -> ScanResult {
    let mut points = Vec::new();

    let p1 = {
        let miss = PointResult {
            name: "LoaderInit",
            state: PointState::NotApplicable,
            code_off: 0,
            positions: Vec::new(),
            code_len: 0,
            };
        match locate_method(img, "PageDownloadInstall", "LoaderInit") {
            None => miss,
            Some(l) => {
                let o = find_all(&l.code, &p1_orig());
                let n = find_all(&l.code, &p1_embedded());
                let legacy = find_all(&l.code, &p1_legacy());
                let state = if !o.is_empty() {
                    PointState::Pending
                } else if !n.is_empty() {
                    PointState::Done
                } else if !legacy.is_empty() {
                    PointState::DoneLegacy
                } else {
                    PointState::Changed
                };
                let positions = if !o.is_empty() {
                    o
                } else if !n.is_empty() {
                    n
                } else {
                    legacy
                };
                PointResult {
                    name: "LoaderInit",
                    state,
                    code_off: l.code_off,
                    positions,
                    code_len: l.code.len(),
                            }
            }
        }
    };
    points.push(p1);

    let p2 = scan_point(
        img,
        "PageDownloadLeftCtor",
        "PageDownloadLeft",
        ".ctor",
        &p2_orig(),
        &p2_new(),
    );
    let v1_set = matches!(
        p2.state,
        PointState::Pending | PointState::Done
    );
    points.push(p2);

    let others: [(&'static str, &str, &str, Pattern, Pattern); 4] = [
        ("LaunchButtons", "PageLaunchLeft", "RefreshButtonsUI", p3_orig(), p3_new()),
        ("ModDisabledDownload", "PageInstanceModDisabled", "BtnDownload_Loaded", p4_orig(), p4_new()),
        ("ResourceSearchButton", "PageResource", "set_PageType", p5_orig(), p5_new()),
        ("EmptyListDownload", "PageSelectRight", "McInstanceListUI", p6_orig(), p6_new()),
    ];
    for (name, ty, method, orig, new) in others {
        let mut r = scan_point(img, name, ty, method, &orig, &new);
        if !v1_set {
            r.state = PointState::NotApplicable;
        }
        points.push(r);
    }

    points.push(scan_p7(img));

    ScanResult { points, v1_set }
}

fn scan_p7(img: &CliImage) -> PointResult {
    let miss = PointResult {
        name: "ModpackInstallButton",
        state: PointState::NotApplicable,
        code_off: 0,
        positions: Vec::new(),
        code_len: 0,
    };
    let located = match locate_method(img, "PageSelectLeft", "McFolderListUI") {
        Some(l) => l,
        None => return miss,
    };
    let code = &located.code;
    let tip_tok = match img
        .find_us_token(|s| s == "在当前选择的 Minecraft 文件夹下安装整合包")
    {
        Some(t) => t,
        None => return miss,
    };
    let tip_pat = pat(&format!(
        "72 {:02x} {:02x} {:02x} 70",
        tip_tok & 0xFF,
        (tip_tok >> 8) & 0xFF,
        (tip_tok >> 16) & 0xFF
    ));
    if find_all(code, &tip_pat).len() != 1 {
        return miss;
    }
    let orig = pat("7e ?? ?? ?? 04 6f ?? ?? ?? 06 6f ?? ?? ?? 0a 73 ?? ?? ?? 0a 25 23 00 00 00 00 00 00 24 40 6f ?? ?? ?? 0a 25 16 6f ?? ?? ?? 0a 6f ?? ?? ?? 0a 26");
    let done = pat("7e ?? ?? ?? 04 6f ?? ?? ?? 06 6f ?? ?? ?? 0a 07 6f ?? ?? ?? 0a 26 2b 03 ?? ?? ?? 2b 03 ?? ?? ?? 2b 03 ?? ?? ?? 00 00 00 00 00 00 00 00 00 00 00");
    let o = find_all(code, &orig);
    let n = find_all(code, &done);
    let (state, positions) = if !o.is_empty() {
        (PointState::Pending, o)
    } else if !n.is_empty() {
        (PointState::Done, n)
    } else {
        (PointState::NotApplicable, Vec::new())
    };
    PointResult {
        name: "ModpackInstallButton",
        state,
        code_off: located.code_off,
        positions,
        code_len: code.len(),
    }
}

fn point<'a>(scan: &'a ScanResult, name: &str) -> &'a PointResult {
    scan.points.iter().find(|p| p.name == name).expect("point")
}

pub fn state_of(scan: &ScanResult) -> crate::PatchState {
    let mut pending = false;
    let mut done = false;
    for p in &scan.points {
        match p.state {
            PointState::Pending => pending = true,
            PointState::Done | PointState::DoneLegacy => done = true,
            PointState::Changed => return crate::PatchState::Unknown,
            PointState::NotApplicable => {}
        }
    }
    if pending && done {
        crate::PatchState::Partial
    } else if pending {
        crate::PatchState::Restricted
    } else if done {
        crate::PatchState::Patched
    } else {
        crate::PatchState::Unknown
    }
}

pub fn apply(img: &mut CliImage, scan: &ScanResult) -> Result<(), String> {
    {
        let p = point(scan, "LoaderInit");
        if p.state == PointState::Pending {
            let dl = p1_dl_field_token(img).ok_or("未找到 DlClientListLoader 字段")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                let orig: Vec<u8> = img.data[start..start + 48].to_vec();
                let mut blk = [0u8; 48];
                blk[0] = 0x7e;
                blk[1..4].copy_from_slice(&dl.to_le_bytes()[..3]);
                blk[4] = 0x04;
                blk[5] = 0x2b;
                blk[6] = 0x29;
                blk[7..48].copy_from_slice(&orig[7..48]);
                img.data[start..start + 48].copy_from_slice(&blk);
            }
        }
    }
    {
        let p = point(scan, "PageDownloadLeftCtor");
        if p.state == PointState::Pending {
            let tok = p2_install_token(img).ok_or("未找到 get_ItemInstall 方法")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                img.data[start + 25] = 0x16;
                img.data[start + 37] = 0x16;
                img.data[start + 44] = 0x16;
                img.data[start + 52..start + 56].copy_from_slice(&tok.to_le_bytes());
            }
        }
    }
    static SITES: [(&str, usize, u8); 4] = [
        ("LaunchButtons", 7, 0x18),
        ("ModDisabledDownload", 0, 0x16),
        ("ResourceSearchButton", 10, 0x16),
        ("EmptyListDownload", 16, 0x16),
    ];
    for (name, off, val) in SITES {
        let p = point(scan, name);
        if p.state == PointState::Pending {
            for &pos in &p.positions {
                let start = p.code_off + pos;
                img.data[start + off] = val;
            }
        }
    }
    {
        let p = point(scan, "ModpackInstallButton");
        if p.state == PointState::Pending {
            let newobj_tok = img
                .find_memberref_token("FrameworkElement", ".ctor")
                .ok_or("未找到 FrameworkElement..ctor")?;
            let set_h = img
                .find_memberref_token("FrameworkElement", "set_Height")
                .ok_or("未找到 FrameworkElement.set_Height")?;
            let set_ihv = img
                .find_memberref_token("UIElement", "set_IsHitTestVisible")
                .ok_or("未找到 UIElement.set_IsHitTestVisible")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                let c = &img.data[start..start + 48];
                let mut blk = [0u8; 48];
                blk[0..15].copy_from_slice(&c[0..15]);
                blk[15] = 0x07;
                blk[16..21].copy_from_slice(&c[42..47]);
                blk[21] = 0x26;
                blk[22] = 0x2b;
                blk[23] = 0x03;
                blk[24..27].copy_from_slice(&newobj_tok.to_le_bytes()[..3]);
                blk[27] = 0x2b;
                blk[28] = 0x03;
                blk[29..32].copy_from_slice(&set_h.to_le_bytes()[..3]);
                blk[32] = 0x2b;
                blk[33] = 0x03;
                blk[34..37].copy_from_slice(&set_ihv.to_le_bytes()[..3]);
                img.data[start..start + 48].copy_from_slice(&blk);
            }
        }
    }
    Ok(())
}

pub fn restore(img: &mut CliImage, scan: &ScanResult) -> Result<(), String> {
    {
        let p = point(scan, "LoaderInit");
        if p.state == PointState::Done {
            let ldstr = p1_ldstr_token(img).ok_or("未找到限制提示文案字符串")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                let data_area_start = start + 7;
                let stsfld_at = data_area_start + 29;
                if img.data[stsfld_at] != 0x80 {
                    return Err("嵌入式数据区校验失败，已拒绝还原".into());
                }
                let a0 = img.data[stsfld_at + 1];
                let mut blk = [0u8; 48];
                blk[0] = 0x72;
                blk[1..4].copy_from_slice(&(ldstr & 0x00FF_FFFF).to_le_bytes()[..3]);
                blk[4] = 0x70;
                blk[5] = 0x7e;
                blk[6] = a0;
                blk[7..48].copy_from_slice(&img.data[data_area_start..data_area_start + 41]);
                img.data[start..start + 48].copy_from_slice(&blk);
            }
        } else if p.state == PointState::DoneLegacy {
            return Err("该文件由 0.2.0 旧版工具打补丁，无法无损还原：请用 .bak 备份覆盖或重新安装官方原版".into());
        }
    }
    {
        let p = point(scan, "PageDownloadLeftCtor");
        if p.state == PointState::Done {
            let tok = p2_mod_token(img).ok_or("未找到 get_ItemMod 方法")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                img.data[start + 25] = 0x18;
                img.data[start + 37] = 0x18;
                img.data[start + 44] = 0x18;
                img.data[start + 52..start + 56].copy_from_slice(&tok.to_le_bytes());
            }
        }
    }
    static SITES: [(&str, usize, u8); 4] = [
        ("LaunchButtons", 7, 0x17),
        ("ModDisabledDownload", 0, 0x18),
        ("ResourceSearchButton", 10, 0x18),
        ("EmptyListDownload", 16, 0x18),
    ];
    for (name, off, val) in SITES {
        let p = point(scan, name);
        if p.state == PointState::Done {
            for &pos in &p.positions {
                let start = p.code_off + pos;
                img.data[start + off] = val;
            }
        }
    }
    {
        let p = point(scan, "ModpackInstallButton");
        if p.state == PointState::Done {
            let newobj_tok = img
                .find_memberref_token("FrameworkElement", ".ctor")
                .ok_or("未找到 FrameworkElement..ctor")?;
            let set_h = img
                .find_memberref_token("FrameworkElement", "set_Height")
                .ok_or("未找到 FrameworkElement.set_Height")?;
            let set_ihv = img
                .find_memberref_token("UIElement", "set_IsHitTestVisible")
                .ok_or("未找到 UIElement.set_IsHitTestVisible")?;
            let add_tok = img
                .find_memberref_token("UIElementCollection", "Add")
                .ok_or("未找到 UIElementCollection.Add")?;
            for &pos in &p.positions {
                let start = p.code_off + pos;
                let mut blk = [0u8; 48];
                blk[0..15].copy_from_slice(&img.data[start..start + 15]);
                blk[15] = 0x73;
                blk[16..20].copy_from_slice(&newobj_tok.to_le_bytes());
                blk[20] = 0x25;
                blk[21..30]
                    .copy_from_slice(&[0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x40]);
                blk[30] = 0x6f;
                blk[31..35].copy_from_slice(&set_h.to_le_bytes());
                blk[35] = 0x25;
                blk[36] = 0x16;
                blk[37] = 0x6f;
                blk[38..42].copy_from_slice(&set_ihv.to_le_bytes());
                blk[42] = 0x6f;
                blk[43..47].copy_from_slice(&add_tok.to_le_bytes());
                blk[47] = 0x26;
                img.data[start..start + 48].copy_from_slice(&blk);
            }
        }
    }
    Ok(())
}

pub fn pat_for_test(spec: &str) -> Pattern {
    pat(spec)
}
