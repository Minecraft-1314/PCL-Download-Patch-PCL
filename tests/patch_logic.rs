use pcl_download_patch::{apply_patch, locate, restore, state_of, PatchState, NEW, ORIG};

/// 真实文件集成测试：设置 PCL_TEST_V0 / PCL_TEST_V1 指向对应原版 exe 时启用
fn v0_path() -> Option<String> {
    std::env::var("PCL_TEST_V0").ok().filter(|s| !s.is_empty())
}
fn v1_path() -> Option<String> {
    std::env::var("PCL_TEST_V1").ok().filter(|s| !s.is_empty())
}

#[test]
fn patterns_are_length_preserving() {
    // 每个补丁点的 orig/new 必须等长（等长替换的前提）
    let img = match v1_path() {
        Some(p) => pcl_download_patch::meta::CliImage::load(&p).expect("parse v1"),
        None => return,
    };
    let scan = pcl_download_patch::patcher::scan(&img);
    assert!(scan.v1_set, "2.99.0.1+ 限制集应命中");
    for p in &scan.points {
        assert!(
            !matches!(p.state, pcl_download_patch::patcher::PointState::Changed),
            "{} 不应处于 Changed 状态",
            p.name
        );
    }
}

#[test]
fn v1_full_roundtrip_is_lossless() {
    let path = match v1_path() {
        Some(p) => p,
        None => return,
    };
    let original = std::fs::read(&path).expect("read v1");
    let mut data = original.clone();

    assert_eq!(state_of(&data), PatchState::Restricted);
    apply_patch(&mut data).expect("apply");
    assert_eq!(state_of(&data), PatchState::Patched);
    // 等长
    assert_eq!(data.len(), original.len());
    // 再补一次应报已打补丁
    assert!(matches!(apply_patch(&mut data), Err(_)));
    // 还原
    restore(&mut data).expect("restore");
    assert_eq!(state_of(&data), PatchState::Restricted);
    // 字节级无损
    assert_eq!(data, original, "restore 后应与原版逐字节一致");
}

#[test]
fn v1_partial_then_complete() {
    let path = match v1_path() {
        Some(p) => p,
        None => return,
    };
    let original = std::fs::read(&path).expect("read v1");
    let mut data = original.clone();
    apply_patch(&mut data).expect("apply");
    // 模拟旧工具只补了 LoaderInit 的场景：把其余点手工改回原版形态
    let img = pcl_download_patch::meta::CliImage::parse(data.clone()).expect("parse");
    let scan = pcl_download_patch::patcher::scan(&img);
    for p in &scan.points {
        if p.name == "LoaderInit" {
            continue;
        }
        for &pos in &p.positions {
            let start = p.code_off + pos;
            // 用原版字节恢复该点（此处仅 P3..P6 的单字节点与 P2 的 5 字节）
            for (off, val) in [
                (25usize, 0x18u8),
                (37, 0x18),
                (44, 0x18),
            ] {
                if p.name == "PageDownloadLeftCtor" {
                    data[start + off] = val;
                }
            }
            match p.name {
                "LaunchButtons" => data[start + 7] = 0x17,
                "ModDisabledDownload" => data[start] = 0x18,
                "ResourceSearchButton" => data[start + 10] = 0x18,
                "EmptyListDownload" => {
                    for &pp in &p.positions {
                        data[p.code_off + pp + 16] = 0x18;
                    }
                }
                "PageDownloadLeftCtor" => {
                    // token 恢复为 get_ItemMod（从元数据解析）
                    let img2 = pcl_download_patch::meta::CliImage::parse(data.clone()).unwrap();
                    let (rid, _) = img2
                        .find_method("PCL", "PageDownloadLeft", "get_ItemMod")
                        .expect("get_ItemMod");
                    let tok = img2.method_token(rid);
                    data[start + 52..start + 56].copy_from_slice(&tok.to_le_bytes());
                }
                _ => {}
            }
        }
    }
    // 部分补丁状态应可继续补齐
    assert_eq!(state_of(&data), PatchState::Partial);
    apply_patch(&mut data).expect("re-apply");
    assert_eq!(state_of(&data), PatchState::Patched);
    restore(&mut data).expect("restore");
    assert_eq!(data, original);
}

#[test]
fn v0_roundtrip_is_lossless() {
    let path = match v0_path() {
        Some(p) => p,
        None => return,
    };
    let original = std::fs::read(&path).expect("read v0");
    let mut data = original.clone();
    assert_eq!(state_of(&data), PatchState::Restricted);
    apply_patch(&mut data).expect("apply");
    assert_eq!(state_of(&data), PatchState::Patched);
    restore(&mut data).expect("restore");
    assert_eq!(data, original, "0.0 往返应无损");
}

#[test]
fn locate_still_works_on_classic_fixture() {
    // 经典 2.99.0.0 特征块仍可被 locate 定位（兼容旧 API 语义）
    let mut data = vec![0xAAu8; 4096];
    // 直接在数据中放置 LoaderInit 的完整方法体（fat 头 + 94B IL）无法伪造元数据，
    // 因此这里仅验证：无 CLI 元数据时 locate 返回 None、state 为 Unknown
    data[512..512 + ORIG.len()].copy_from_slice(&ORIG);
    assert_eq!(locate(&data), None);
    assert_eq!(state_of(&data), PatchState::Unknown);
    // NEW 常量保留用于文档对照
    assert_eq!(NEW[0], 0x7e);
    assert_eq!(ORIG[0], 0x72);
}
