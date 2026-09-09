# Music Native Names

**在原生 Apple Music 中显示歌曲、歌手和专辑的原文名称。**

[English](README.md) · 简体中文

基于 Rust 和 AppKit 的轻量 macOS 菜单栏工具。通过 Apple 官方目录验证同一录音或发行版本，
在证据充分时替换显示名称，保留现有 Apple Music 账号和商店地区。

[下载](https://github.com/LogicSense1/music-native-names/releases) ·
[更新记录](CHANGELOG.md) · [反馈问题](https://github.com/LogicSense1/music-native-names/issues)

> **实验性兼容范围：** 仅支持 Apple Silicon；当前原生桥接依赖系统**已经关闭 SIP**，并安装
> Apple Command Line Tools。使用 Music 私有运行时接口，并非 Apple 官方插件。
> 应用和安装器不会修改 SIP 或其他系统保护。本机已验证 macOS 27.0（26A5425a），
> 尚未确认其他 macOS/Music 版本的运行兼容性。

## 功能

- 在支持的目录页面、专辑详情、迷你播放器和“播放中”界面显示原文名称。
- 使用 Apple 等价映射、录音时长、内容分级和可用的歌手身份信息，避免替换成另一首录音的名称。
- 原本为英文的名称可以保留英文；无法唯一确认时保留已有显示。
- 支持暂停/恢复、重新连接、登录自启、清空缓存和按天保留日志。
- 一个常驻 Rust 进程，无需常驻 Python、浏览器或调试器。

目前从**美区商店**资源出发，在歌手或发行证据支持时查询日本、韩国、中国大陆和台湾地区目录。
不翻译歌词、播放列表或已经组合好的推荐栏目文案。Apple 没有统一的“原文语言”字段，
歌手来源仍依赖目录信息推断，不能保证所有作品都可识别。

## 安装

1. 从 [Releases](https://github.com/LogicSense1/music-native-names/releases) 下载
   `MusicNativeNames-<版本>-macos-arm64.zip`。
2. 完整解压，打开 `install.command`。
3. 打开 Music，工具会自动连接；首次安装原生桥接时会短暂暂停 Music 进程。

安装器会先检查运行条件，再替换旧版。应用安装至 `~/Applications/Music Native Names.app`，
默认启用登录自启。旧版本保留在同一目录，名称包含时间戳 `.previous-…`。

发布包使用本地 ad-hoc 签名，**没有 Developer ID 签名或公证**；macOS 可能阻止下载的应用运行。
也可以按下方步骤从源码构建。不要仅为运行这个实验性工具而降低系统保护。

## 菜单栏状态

菜单栏只显示符号和颜色，不显示汉字。不同图形和辅助功能标签也能区分状态，不依赖颜色单独传达信息。

| 符号 | 颜色 | 状态 |
| --- | --- | --- |
| 音符 | 绿色 | 已连接并启用 |
| 循环箭头 | 橙色 | 正在连接 |
| 时钟 | 橙色 | 等待 Music |
| 暂停圆圈 | 灰色 | 已暂停 |
| 感叹号圆圈 | 红色 | 连接或目录查询异常 |

悬停可查看详细状态。菜单提供启用原文显示、打开 Music、登录自启、清空缓存、重新连接和退出。
**Log** 子菜单可打开日志文件夹，设置保留 1、7、14、30 天，默认 7 天。
**关于 Music Native Names…** 显示版本、MIT 许可证和 GitHub 项目链接。
正常退出会先恢复 Music 的原生方法。

## 构建与测试

需要 Apple Silicon Mac、通过 rustup 安装的 Rust、Apple Command Line Tools，以及用于检查发布包的
Python 3。编译器版本由 `rust-toolchain.toml` 固定，依赖版本由 `Cargo.lock` 固定。

```sh
git clone https://github.com/LogicSense1/music-native-names.git
cd music-native-names
./build.command
./scripts/package.sh --no-build
```

应用输出至 `dist/Music Native Names.app`；安装 ZIP 和 `SHA256SUMS` 同样输出至 `dist/`。
运行 `./install.command` 安装本地构建。

```sh
./scripts/test.sh
cd dist
shasum -a 256 -c SHA256SUMS
```

测试包含格式检查、Rust 单元测试、Clippy、原生时长断言和 arm64e 桥接语法检查。
构建和 CI **不会连接 Music，也不要求关闭 SIP**；实际运行和界面验收需要兼容的本机环境。
开发诊断见 [Development](docs/DEVELOPMENT.md)，实现细节见 [Architecture](docs/ARCHITECTURE.md)。

## 数据与隐私

只替换名称字段，不改写歌曲 ID、播放参数、音频地址、封面、歌词或账号信息。
不会修改磁盘上的官方 Music.app，也不会把名称改动写入音乐资料库。

目录查询使用 Apple 网页公开分发的目录凭据，仅保存在内存，不读取账号 token 或浏览器 cookie。
该接入方式并非稳定的公开集成接口，失效时保留原名；项目没有分析统计上报接口。

| 本地数据 | 路径 |
| --- | --- |
| 设置与缓存 | `~/Library/Application Support/MusicNativeNames/` |
| 每日日志 | `~/Library/Logs/MusicNativeNames/` |
| 登录启动项 | `~/Library/LaunchAgents/local.music-native-names.rust.plist` |

本地服务只监听 `127.0.0.1:18090`，不对网络开放，也没有逐客户端认证；不要暴露或转发该端口。
日志记录运行数量和错误，不保存完整目录响应或凭据。默认保留最近七个自然日，每天约 4 MiB 上限。

## 排查与卸载

- **等待或连接中：** 打开 Music，等待首次桥接安装完成。
- **红色状态：** 查看 Log，确认运行条件，再选择“重新连接 Music”。
- **部分名称仍为罗马字：** 可能来自上游缓存、未覆盖的界面、歧义匹配或缺失的地区发行。
  解决连接问题后可以重新打开页面。
- **系统更新后连接失败：** 暂停工具，并在反馈中提供 macOS/Music 版本；私有接口可能随系统更新变化。

运行 `uninstall.command` 会停用登录自启、恢复已安装的方法并将应用移入废纸篓。
设置、缓存和日志保留，便于恢复。

## 贡献与许可证

贡献方式见 [CONTRIBUTING.md](CONTRIBUTING.md)，项目采用 [MIT 许可证](LICENSE)。
发布包附带依赖许可说明。Apple Music 及相关商标归 Apple Inc. 所有；本项目独立开发，
与 Apple 无隶属关系，也未获得 Apple 背书。

## 支持与打赏

如果这个项目对你有帮助，欢迎 Star、反馈可复现的问题，或提交改进。

**打赏暂未开通。** 可验证的收款链接和支付二维码准备好后会放在这里；
预留方案见 [Support](docs/SUPPORT.md)。
