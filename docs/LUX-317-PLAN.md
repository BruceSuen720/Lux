# LUX-317 统一登录背景插件与自定义图片

> 状态：已确认，实施中；2026-09-30

## 目标与边界

新建唯一插件 `org.lux.login-background`，插件配置选 Bing 每日图、TMDb 日榜或单张自定义上传图片。保持与 `org.lux.tmdb` 元数据插件独立。Bing/TMDb 原 provider 合同、许可提示、榜单选择和宿主 HERO_IMAGE 布局不变；Bing 个人用途和 TMDb 非商业许可确认保留为不同、默认关闭的开关。自定义图需独立确认公开展示权。

自定义图片规则：单张，JPEG/PNG/WebP，最多 5 MiB 和 20 MP；校验真实文件格式和尺寸、原字节保存，不压缩或重编码。上传成功后原子切换新图并清理旧资源；出错时仍保留旧资源。文件由 Lux 存放在配置目录的专用目录，插件进程无文件系统访问。manifest 配置字段 `type: image` 只允许 `org.lux.login-background` 声明一个，且配置值为 `sha256:<64 位小写十六进制>` opaque ID，不是路径或 URL。

上传 API：`PUT /api/v1/admin/plugins/{plugin_id}/config/image/{field_key}`；管理员鉴权并验证 CSRF；服务端校验 body 大小和 manifest field。插件 RPC 的 image URL 固定为 `/api/v1/auth/login-background/custom-image`，仅当 manifest ID 精确等于 `org.lux.login-background` 且声明 image 字段时允许。该 GET/HEAD 同源路由只有插件已安装、启用、可用且服务器选择 `PLUGIN:org.lux.login-background`、plugin config `source=CUSTOM_IMAGE`、`customImageRightsConfirmed=true` 时服务文件；rights gate 由 Lux 宿主独立检查，不依赖插件守约。其他情况 404。返回 sniff 后 MIME、`nosniff`、强 ETag、`Cache-Control: no-cache, must-revalidate`；匹配 If-None-Match 时 304，HEAD 无 body。其它插件仍须使用 HTTPS 且命中 manifest `imageHosts`。

文件按 content SHA-256 保存于固定目录；路径只从经过格式验证的 digest 和已检测格式生成，不用上传文件名。新文件先写唯一临时文件，尺寸/格式校验完成后 rename，再更新 opaque config ID；更新失败删除新文件并保留旧 ID/文件。成功后只保留当前 hash 文件；过期中断留下的临时文件/旧 hash 在后续成功替换时清理。

旧插件切换：新包和双架构 Release 已成功、索引可用之后，从活动目录移除 `org.lux.bing-daily-background` 与 `org.lux.tmdb-trending-background`，删除其 GitHub Release 与 release tags，包含 Bing 0.1.0、TMDb 0.1.0/0.1.1 ZIP。保留 Git commit 历史；不自动卸载用户已有 Lux 实例的旧包。用户需安装新插件、选择旧来源对应模式、重新确认许可、切换服务器来源，验证后手动卸载旧插件。旧许可确认不迁移。

## 技术栈及目录

- Lux host: Rust/Axum、已有 `image 0.25`、Plugin SDK 与 cache。无 DB BLOB/migration；handler 管鉴权与 DTO，应用 service 负责校验/文件写入。
- Web: React + TypeScript 插件配置 image file control 和上传 API client；禁止输入自定义 URL。
- Lux-plugins: Rust standalone process，单 manifest/binary，按 `source` 只调用所选 provider。
- 主要文件：Lux 的 `src/application/plugin_protocol.rs`、`src/application/plugins.rs`、新 `src/application/login_background_assets.rs`、`src/application/mod.rs`、`src/api/admin.rs`、`src/api/admin_handlers.rs`、`src/api/users.rs`、`tests/plugin_protocol.rs`、`tests/login_background.rs`、新增资源测试、`docs/PLUGIN-SDK.md`、`docs/API.md`；Web 配置页与测试；外部 Lux-plugins 的新 binary/manifest、provider tests、catalog tests、CI/release workflow 与迁移说明。

## 增量与阶段门

### 阶段 A：SDK 和 host-owned 图片资源（已通过）

1. SDK image field、digest config validator、fixed same-origin route allowlist；协议测试及 Plugin SDK 文档。
2. `login_background_assets` 应用服务：size/format/dimensions/hash、安全路径、临时写入/rename、回滚、旧资源清理、ETag。service 单测先行。
3. admin raw upload route + public GET/HEAD route；覆盖 CSRF/认证、plugin config field、安装/启用/可用/source gate、404、MIME、nosniff、ETag/304、HEAD。更新 API docs。

阶段 A 验证：`cargo test --locked --test plugin_protocol --test plugins --test login_background` 加新 asset-service integration target；`cargo fmt --all -- --check`；变更 crate 的 Clippy。结束后停下，等项目所有者确认，再进入阶段 B。

### 阶段 B：配置 UI 与统一 provider（已通过；2026-09-30）

4. Admin Plugins 通用配置页 image 字段：单选/替换、已上传状态、许可/公开告知、错误与无障碍 feedback；API types/client/tests。
5. Lux-plugins 新 unified binary+manifest；复用现 Bing/TMDb 行为及 mock 测试；模式选择单一，许可确认分别 fail closed，custom 不联网。
6. 新插件 validation pipeline: x86_64/aarch64 binaries 与 ZIP manifest/hash 校验。新 Release 可用前不删除旧 packages。

阶段 B 验证：Web 全量测试（75 个文件、540 项）及 build 通过；插件 Rust provider/SDK 测试、fmt、所选 Clippy、共享 TMDb client tests、Python 登录背景 catalog/fixture tests 通过；GitHub Actions run [36738619827](https://github.com/Qoo-330ml/Lux-plugins/actions/runs/36738619827) 的 x86_64/aarch64 build、ZIP manifest/binary hash 校验与 artifact upload 全部成功。Playwright 验证因本机 Lux API 未运行，不能证明登录态 Admin Plugins 页面实际交互。结束后停在阶段 C 前，等待项目所有者 cutover 确认。

### 阶段 C：active catalog 与旧 release 清理

7. 将新 ID 加入 plugins.json，移除两个旧 ID；生成 index，实时检查只有新 ID。
8. 精确核验并删除两旧 GitHub Release/tag 与对应资产；任何目标不符则停下，不模糊清理。
9. 发布手动迁移文档，确认旧 release/tag 不存在、新目录包可下载；不连接远端 Lux 主机。

阶段 C 验证：主 release workflow 成功、index URLs+hashs 验证，两个旧 ID 不在 index，旧 releases/tags/assets 不存在，新包两架构可取。以实际部署验证区分 CI 结果。

## 测试策略

- Rust: manifest/image field scope、asset ID shape、reserved URL、size/magic/format/dimensions、symlink/path safety、atomic replacement/rollback/orphan cleanup、admin+CSRF、source state gate、ETag/304/HEAD、404/fallback。
- Provider: mock Bing 与 TMDb API；请求/query、first valid movie/tv backdrop、permission/consent gates、source isolation、custom no network、bad response fallback。
- Web: 选 source、分开的 consent、单图上传替换/失败、公开说明、登录图呈现与失效回退、桌面/窄屏 Playwright。
- Release: 两平台构建、manifest/entrypoint/package/hash、main index/update, old release removal.

## 代码风格示意

特殊同源 URL 必须逐字 allowlist，不做前缀或可解析 URL 级放行：

```rust
let is_custom_asset = manifest.id == UNIFIED_LOGIN_BACKGROUND_PLUGIN_ID
    && image_url == LOGIN_BACKGROUND_CUSTOM_IMAGE_PATH;
```

这只决定 RPC contract；服务路由仍必须独立检查管理员选择、插件状态和有效 opaque asset ID。

## 命令

- Lux target：`CARGO_TARGET_DIR=/Volumes/Toshiba/mywork/Lux/target cargo test --locked --test plugin_protocol --test plugins --test login_background`
- Lux formatting：`cargo fmt --all -- --check`
- Lux lint：`CARGO_TARGET_DIR=/Volumes/Toshiba/mywork/Lux/target cargo clippy --locked --all-targets --all-features -- -D warnings`
- Web: `pnpm --dir web test -- --run web/tests/admin-settings.test.tsx web/tests/plugin-library.test.ts` and `pnpm --dir web build`
- Lux-plugins: `cargo fmt --all -- --check`, selected provider/catalog tests, plus main workflow's x86_64/aarch64 build.

## 当前增量计划

宿主阶段 A 已完成增量：

- A1：SDK image config、SHA-256 ID 与固定 RPC URL。`plugin_protocol` 33 项及 image config unit test 通过；fmt 通过。
- A2：`login_background_assets` 单图文件服务。8 项单测覆盖原字节/三格式、5 MiB/20 MP、有界 hash、幂等、prune 和符号链接拒绝；`cargo clippy --locked --lib --all-features -- -D warnings` 通过。
- A3：PluginService 串行化配置写入，只由 upload API 设置 opaque image ID；普通 JSON config 不可伪造资源 ID。
- A4：管理员上传与公开 GET/HEAD 同源路由。集成覆盖管理员/CSRF、固定 path 与 source mode gate、ETag/304、HEAD、5 MiB、格式校验、替换清理和 disabled 404。

定向验证：`cargo test --locked --test plugin_protocol --test plugins --test login_background`（45 tests）通过；`cargo test --locked --lib application::plugins::plugin_update_tests`（6 tests）和 `cargo test --locked --lib application::login_background_assets::tests`（8 tests）通过。`cargo build --locked`、全目标 Clippy（`-D warnings`）、fmt 与 `git diff --check` 通过。

全局阶段门：最初 `tests/shutdown.rs::unix::luxd_exits_cleanly_on_sigterm_after_startup` 在 10 秒内未观测到 child listener；相同失败在干净的 `test` branch control worktree 可复现。临时打开 stdout 后，startup log 证明迁移、恢复任务清理及 HTTP bind 总共约 12 秒，早于 30 秒测试门限正常启动；同一二进制手动运行也能返回 health 200。根因是 shutdown 测试 10 秒启动门限小于本机冷启动耗时，将其只调整为 30 秒。没有更改服务启动行为。调整后 `cargo test --locked --test shutdown ... -- --exact` 连续通过两次；完整 `cargo test --locked --all-targets -- --test-threads=1` 完成且集成目标通过，638 个 library tests 通过、10 个环境/性能专项标记 ignored。结合 `cargo build --locked`、全目标 Clippy、fmt 和 diff 检查，阶段 A 全局门通过。项目所有者要求“修复然后继续”，因此进入阶段 B。

Lux 主工作目录里有并行任务的未提交修改；LUX-317 在独立 managed worktree 实现，只精确暂存本任务文件。

### 阶段 B 实施记录

- Lux Web API：增加原始图片上传 client 与响应类型；`AdminPluginsPage` 渲染 manifest 声明的单图替换、上传状态/错误，以及各自独立且默认关闭的 Bing、TMDb、自定义公开展示许可确认。普通 JSON 配置保存不提交宿主管理的 image hash。相关 API/UI 定向测试 80 项通过。
- Lux-plugins SDK：只允许 `org.lux.login-background` 声明一个可选、非敏感、无默认值的 image 字段，并导出固定 same-origin 资源路径。
- 新插件 `org.lux.login-background`：配置选择 `BING_DAILY`、`TMDB_TRENDING`、`CUSTOM_IMAGE`；每次仅运行所选 provider。Bing 复用 `HPImageArchive` 行为；TMDb 复用内嵌 fallback key 并只读取 `/3/trending/all/day` 首个有效电影/剧集 backdrop；自定义模式仅返回宿主托管图片路由，使用 HERO_IMAGE 全幅布局且不创建网络 client。
- Lux-plugins provider/SDK 测试 25 项通过，Bing/TMDb mock HTTP 均通过；fmt、所选 Clippy、共享 TMDb client tests 及登录背景 catalog/fixture tests 通过。
- 插件工作树的分支 `codex/unified-login-background` 已推送；workflow run `36738619827` 成功完成 x86_64/aarch64 构建、测试、Clippy、ZIP/manifest/hash 校验与 artifact upload。没有更新活动 `plugins.json`、发布 Release、删除旧包或旧 tag。
- Lux Web 全量测试最终 75 个文件、540 项通过；有一次首次全量运行的首页轮播 aria-current 断言失败，单测复跑及随后完整重跑通过。`pnpm --dir web build` 与锁文件安装通过；Playwright 因本机没有 Lux API (`127.0.0.1:8097`) 只能验证到前端壳层，登录态插件配置页的真实浏览器流程尚未证实。
- 外部仓库整个 `test_release_workflow.py` 有一个既有 webhook manifest 断言 `KeyError: bodyTemplate`；其余五个方法单独通过。登录背景专用 Python catalog/fixture suite 全部通过。

阶段 B 阶段门已通过。外部仓库全量 `test_release_workflow.py` 单独运行仍有一个与本任务无关的 webhook 旧断言 `KeyError: bodyTemplate`；该 workflow 不调用此脚本，登录背景专用 Python suite 与双架构 CI 均通过。首次 Web 全量测试曾有一次首页轮播异步断言失败，定向复跑和随后完整重跑通过。Playwright 访问本机前端时，由于 Lux API `127.0.0.1:8097` 未运行，收到 setup status 500；真实登录态配置页交互未验证。

阶段 C 尚未开始：没有改动活动 `plugins.json`/`index.json`，没有创建正式 Release，也没有删除旧 release/tag/package。等待项目所有者明确确认 cutover。
