# PCL Download Patch PCL 下载限制补丁

## 重要公告（必读）

- 本工具只改写 `Plain Craft Launcher 2.exe` 内的 IL 字节，不改动 PCL 其他文件
- **版本自适应**：工具解析 PCL 的 .NET 元数据并按「类型名 + 方法名 + IL 结构」定位修改点，不依赖版本号、文件哈希或具体令牌值。2.99.0.1 之后的版本只要官方未改动相关方法的实现结构，即可直接使用
- 适配 PCL 2.99.0.0 及以上版本：2.99.0.0 恢复版本列表加载器（1 处）；2.99.0.1 起额外恢复全部下载入口（7 处）
- 首次修改前会自动在 PCL 同目录生成 `Plain Craft Launcher 2.exe.bak` 备份，已有备份时不覆盖，可用 `restore` 一键还原
- 特征不匹配时工具会拒绝修改并保留原文件，不会造成损坏
- 低于 2.99.0.0 的版本本就没有下载限制，无需使用本工具
- 请在 PCL 完全退出后执行，文件被占用时无法写入
- 本工具不联网、无任何运行时依赖

## 项目介绍

PCL2 为遵守相关法律法规移除了「下载原版游戏」功能，两个版本的手法不同：

| 版本 | 官方手法 | 本工具恢复内容 |
|------|----------|----------------|
| 2.99.0.0 | 编译期常量折叠：下载页的版本列表加载器被替换为固定抛出限制提示的加载器 | 还原该调用为真正的版本列表加载器（1 处） |
| 2.99.0.1 | 在上述基础上进一步删除全部下载入口（官方更新日志：「删除：MC 下载功能相关入口」），并在「社区资源」标题上加了隐藏逻辑 | 恢复下载页「原版游戏」入口与「社区资源」标题、版本列表加载器、启动页「下载游戏」按钮、实例列表「整合包安装」入口等全部 8 处 |

## 使用方法

把 `PCLPatch.exe` 放进 PCL 目录（与 `Plain Craft Launcher 2.exe` 同级）后执行：

| 命令 | 作用 |
|------|------|
| `PCLPatch.exe patch` | 打补丁（省略命令名时同样执行） |
| `PCLPatch.exe restore` | 还原为原版限制状态 |
| `PCLPatch.exe status` | 查看当前状态与适配版本 |
| `PCLPatch.exe help` | 显示帮助 |

也可显式指定路径：`PCLPatch.exe patch "D:\PCL\Plain Craft Launcher 2.exe"`

直接双击 `PCLPatch.exe` 会打开交互式菜单窗口（↑↓ 方向键或数字选择，中英双语随时切换），打补丁、还原、查状态全部在一个 exe 里完成；把 PCL 主程序拖到 `PCLPatch.exe` 图标上同样会打开菜单并自动选中该文件（首次运行选择语言，之后自动记住）

退出码：`0` 成功；`1` 失败（文件不存在、特征不匹配、特征歧义或读写错误）

## 下载

| 产物 | 说明 |
|------|------|
| `dist\PCLPatch.exe` | 单文件命令行工具，解压即用 |

## 恢复明细

| # | 位置（类型.方法） | 官方限制 | 恢复为 |
|---|------------------|----------|--------|
| 1 | `PageDownloadInstall.LoaderInit` | 版本列表加载器 = 抛「不再提供 MC 下载功能」异常的 LoaderTask | `ldsfld ModDownload::DlClientListLoader`，官方原 48 字节 IL 保留在跳转死区内以便无损还原 |
| 2 | `PageDownloadLeft..ctor` | 「原版游戏」入口 Collapsed、「社区资源」标题 Collapsed、默认子页=模组、默认选中模组 | 入口与标题可见、默认子页=原版游戏、默认选中「原版游戏」 |
| 3 | `PageLaunchLeft.RefreshButtonsUI` | 无版本时按钮状态号=1（伪装加载中，下载入口不可达） | 状态号=2（「下载游戏」按钮分支） |
| 4 | `PageInstanceModDisabled.BtnDownload_Loaded` | 下载按钮无条件 Collapsed | 无条件 Visible |
| 5 | `PageResource.set_PageType` | 整合包页「搜索安装 Mod 包」按钮无条件 Collapsed | 按 `PageType==2` 显示 |
| 6 | `PageSelectRight.McInstanceListUI` | 空版本列表的下载按钮在两个分支里均 Collapsed | 两个分支均恢复显示 |
| 7 | `PageSelectLeft.McFolderListUI` | 实例列表的「整合包安装」按钮整句 `PanList.Children.Add` 被删除（22 字节） | 等长借用紧随其后的间隔元素 Add 槽位改放该按钮；被删指令的令牌以跳板形式嵌入 nop 区供无损还原 |
| 8 | `PageDownloadResourceDetail.InstallOrSave_Click` | 整合包安装流程被官方摘除，版本行点击无处理入口 | 版本行点击按 `ResourceVersion.ResourceType` 分流：整合包（=2）走下载并安装整合包，Mod / 数据包 / 资源包 / 光影包走 `Save_Click` 下载资源文件；「另存为」按钮始终为 `Save_Click` |

所有修改均为等长 IL 替换，不改变文件大小与后续所有偏移。修改点是 IL 而非机器码。

## 整合包安装流程的适用范围

补丁点 8 由内置的 `ilpatch` 组件（dnlib 实现）完成。整合包安装流程只在版本条目本身的资源类型为整合包（`ResourceTypes = 2`）时启用，Mod、数据包、资源包、光影包的版本条目点击后走正常的资源文件下载，不会再触发整合包安装。

## 还原原理（无损往返）

补丁 1 采用「跳转死区」结构：恢复形态的 48 字节 = `ldsfld DlClientListLoader` + `br.s` + 官方原版 IL 的后 41 字节（跳转死区，永不执行）。还原时从死区取回官方 IL 主体，配合从 #US 流按名称解析的加载器名字符串即可逐字节重建官方原版，`restore` 后文件与官方原版完全一致（已通过 SHA-256 验证）

由 0.2.0 旧版工具打过的文件（LoaderInit 处为 43 个 `00` 的旧形态）功能不受影响，但无法无损还原，`restore` 时会提示改用 `.bak` 备份或重装官方原版

## 手动补丁指南（不使用本工具）

面向希望理解或手动完成修改的专业人士。工具按结构定位，手动操作建议用 dnSpy 等 IL 编辑器：

1. 完全退出 PCL，复制一份 `Plain Craft Launcher 2.exe` 作为备份
2. 用 dnSpy 打开 exe，定位下表中的类型与方法
3. 按下表修改（编辑方法 IL，注意保持栈平衡与长度不变；dnSpy 编辑方法可勾选「保持最大栈」并手动补 `nop` 对齐）

| 版本 | 修改点 | 修改内容 |
|------|--------|----------|
| 全部 2.99.0.0+ | `PageDownloadInstall.LoaderInit` | 方法体中 `ldstr ...; ldsfld ...; brfalse.s; ldsfld; br.s; ldsfld; ldtoken; newobj; dup; stsfld; ldnull; ldc.i4.2; newobj` 的 48 字节块整体替换为 `ldsfld <ModDownload.DlClientListLoader 的字段令牌>` 后跟 43 个 `nop` |
| 2.99.0.1+ | `PageDownloadLeft..ctor` | `ItemInstall`/`LabResource` 的 `ldc.i4.2`（Collapsed）改为 `ldc.i4.0`（Visible）；第二处 `ldc.i4.2; stfld PageID` 改为 `ldc.i4.0`（Default）；`callvirt get_ItemMod` 改为 `callvirt get_ItemInstall` |
| 2.99.0.1+ | `PageLaunchLeft.RefreshButtonsUI` | `callvirt get_McInstanceSelected; brtrue.s +4;` 之后的 `ldc.i4.1` 改为 `ldc.i4.2` |
| 2.99.0.1+ | `PageInstanceModDisabled.BtnDownload_Loaded` | 方法体开头的 `ldc.i4.2; stloc.0` 改为 `ldc.i4.0; stloc.0` |
| 2.99.0.1+ | `PageResource.set_PageType` | `pop; pop` 之后的 `ldc.i4.2` 改为 `ldc.i4.0` |
| 2.99.0.1+ | `PageSelectRight.McInstanceListUI` | 两处 `callvirt get_BtnEmptyDownload` 之后的 `ldc.i4.2` 均改为 `ldc.i4.0` |

4. 保存模块，启动 PCL 验证：「下载 → 原版游戏」入口恢复，版本列表正常加载
5. 还原：用备份覆盖回去

### 注意事项

- 官方更新后若上述方法的实现结构变化，工具会拒绝修改并提示，此时不要强行修改，也不建议按偏移盲改
- 带数字签名的程序集修改后签名会失效

## 项目贡献者

| 贡献者 | 贡献内容 |
|--------|----------|
| Minecraft-1314 | 全部代码开发 |

欢迎通过 Issues 或 PR 加入贡献者列表

## 许可协议

本项目采用 MIT 许可证，详情见 [LICENSE](LICENSE) 文件
