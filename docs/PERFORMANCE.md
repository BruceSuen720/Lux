# Lux 性能记录

本文档记录可重复的基准结果。没有硬件、数据集、命令和提交信息的数字不作为验收证据。

## 基准目标

规格目标：10,000 部电影、50,000 集剧集；数据库预热、单页 50 条、扫描同时运行。API 目标为首页 p95 < 400 ms、媒体库首屏 p95 < 300 ms、搜索 p95 < 500 ms、详情 p95 < 200 ms、继续观看 p95 < 300 ms、缓存图片 p95 < 150 ms，扫描期间前台 p95 < 1 s 且不超过空闲时 2 倍。

## 记录模板

| 日期 | 提交 | 硬件/架构 | 数据集 | 命令 | 场景 | p50 | p95 | 错误率 | 内存 | 备注 |
|---|---|---|---|---|---|---:|---:|---:|---:|---|
| 2026-09-08 | 12b20f2d（基于 7f683012） | macOS ARM64 (`uname -m=arm64`) | SQLite；先执行 1–117 迁移并写入代表性扫描数据，再执行 118 | `cargo test --locked --test storage scan_index_compaction_preserves_existing_rows_during_upgrade` | 已有数据库升级与扫描索引压缩回归 | 1 passed | - | 0% | - | `reconciliation_scan_entries` 数据、`scan_job_targets` 数据和外键检查均保留；只记录迁移正确性，不外推 PostgreSQL WAL、数据库体积或 NAS/x86_64 性能 |
| 2026-09-08 | 6bd90d21 | macOS ARM64 (`uname -m=arm64`) | SQLite；1,025 条无个人数据扫描发现路径 | `cargo test --locked --lib storage::repository::repository_tests::reconciliation_entries_use_scan_safe_batches -- --exact` | 扫描发现中间表批量写入 | 11 → 6 条 DML | - | 0% | - | 扫描专用批次从 100 增至 200；每条语句最多 800 个绑定参数，低于 SQLite 历史 999 参数上限；约减少 45.5% 批量写入语句；不外推 PostgreSQL WAL 或 NAS/x86_64 性能 |
| 2026-08-29 | 0c621c60 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 5,123 / 1,291 / 1,421 ms；56 / 104 ms | 5,123 / 1,291 / 1,421 ms；61 / 302 ms | 0% | - | release；前台 50 请求 p95 203 ms，`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；后台剧集/混合库批量索引、指纹有界并发、混合分类 NFO 缓存、超大目录发现分块由 `scanning_jobs` 集成测试覆盖；本机 ARM64，不外推 NAS/x86_64 |
| 2026-08-02 | 740de3c | macOS ARM64 (`aarch64-apple-darwin`), Rust 1.97.1 | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次全库扫描 | 14,104 ms | 14,104 ms | 0% | - | 60,000 条目；release 模式；未触发 NFO/ffprobe |
| 2026-08-02 | 740de3c | macOS ARM64 (`aarch64-apple-darwin`), Rust 1.97.1 | 同上 | `./scripts/run-performance.sh` | 无变化全库重扫 | 4,061 ms | 4,061 ms | 0% | - | 60,000 条目全部 fingerprint 命中并跳过 |
| 2026-08-02 | 740de3c | macOS ARM64 (`aarch64-apple-darwin`), Rust 1.97.1 | 同上 + 单目录新增 100 文件 | `./scripts/run-performance.sh` | 单目录增量（200 文件目录） | 31 ms | 31 ms | 0% | - | 100 个既有文件跳过，100 个新增文件入库；未标记其他路径 missing |
| 2026-08-02 | 740de3c | macOS ARM64 (`aarch64-apple-darwin`), Rust 1.97.1 | 同上 | `./scripts/run-performance.sh` | 扫描期间 50 个管理员库列表请求 | 4 ms | 4 ms | 0% | - | `foregroundDuringScan=true`；目标前台 p95 < 1,000 ms |
| 2026-08-03 | 50a9e09 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 16,526 / 4,131 / 36 ms | 16,526 / 4,131 / 36 ms | 0% | - | release；前台 50 请求 p95 8 ms，`foregroundErrors=0`；fixture 摘要同上 |
| 2026-08-03 | c23a757 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 23,574 / 7,520 / 1,300 ms | 23,574 / 7,520 / 1,300 ms | 0% | - | release；前台 50 请求 p95 11 ms，`foregroundErrors=0`；用户状态列表改为分块批量查询；fixture 摘要同上 |
| 2026-08-03 | df28a97 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 21,394 / 4,307 / 41 ms | 21,394 / 4,307 / 41 ms | 0% | - | release；前台 50 请求 p95 10 ms，`foregroundErrors=0`；fixture 摘要同上 |
| 2026-08-03 | 8796365 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 38,024 / 10,392 / 41 ms | 38,024 / 10,392 / 41 ms | 0% | - | release；前台 50 请求 p95 10 ms，`foregroundErrors=0`；本机负载导致扫描耗时波动；fixture 摘要同上 |
| 2026-08-03 | ba39b1d | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 17,232 / 4,364 / 42 ms | 17,232 / 4,364 / 42 ms | 0% | - | release；前台 50 请求 p95 11 ms，`foregroundErrors=0`；fixture 摘要同上 |
| 2026-08-03 | b42a133 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量 | 22,506 / 6,481 / 41 ms | 22,506 / 6,481 / 41 ms | 0% | - | release；前台 50 请求 p95 12 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；fixture 摘要同上 |
| 2026-08-08 | f3f0d460 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 6,459 / 7,987 / 46 ms；336 / 131 ms | 6,459 / 7,987 / 46 ms；340 / 7,287 ms | 0% | - | release；扫描期间前台 50 请求 p95 42 ms；目录列表 50 并发 p95 340 ms；搜索单次 131 ms、50 并发 p95 7,287 ms；`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；fixture 摘要同上 |
| 2026-08-09 | c022fcac | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 6,043 / 5,634 / 46 ms；301 / 4,823 ms | 6,043 / 5,634 / 46 ms；306 / 4,848 ms | 0% | - | release；扫描期间前台 50 请求 p95 50 ms；目录列表 50 并发 p95 306 ms；搜索单次 116 ms、50 并发 p95 4,848 ms；`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；fixture 摘要同上；该脚本仍直接调用 `LibraryScanner`，持久化后台任务另由扫描任务集成测试覆盖 |
| 2026-08-10 | 5e0bef61 | macOS ARM64 (`aarch64-apple-darwin`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 6,234 / 7,593 / 47 ms；225 / 3,161 ms | 6,234 / 7,593 / 47 ms；366 / 6,103 ms | 0% | - | release；扫描期间前台 50 请求 p95 49 ms；目录列表 50 并发 p95 366 ms；搜索单次 83 ms、50 并发 p95 6,103 ms；目录聚合限制为 16 个执行、64 个总在途请求；`foregroundErrors=0`；未测量 macOS RSS，不能验证 Linux/glibc arena 回收 |
| 2026-08-15 | 8b2dca5a（工作树） | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 183,854 / 8,538 / 55 ms；1,139 / 2,799 ms | 183,854 / 8,538 / 55 ms；1,840 / 4,605 ms | 0% | - | release；扫描期间前台 p95 93 ms；`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；本次开发机负载下首扫明显慢于历史记录，不能据此归因于本改动或外推 NAS 性能 |

| 2026-08-21 | 7e0578a5 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,676 / 6,383 / 1,331 ms；35 / 3,120 ms | 4,676 / 6,383 / 1,331 ms；40 / 4,172 ms | 0% | - | release；电影身份与目录预取、filesystem/media_items/media_sources 批量写入；扫描期间前台 p95 154 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-21 | 60f3028c | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 同上 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,687 / 6,406 / 1,393 ms；38 / 2,721 ms | 4,687 / 6,406 / 1,393 ms；44 / 4,219 ms | 0% | - | release；有界文件准备并发、目录 provider ID 批内复用、后台默认批次 100；扫描期间前台 p95 150 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；与前一阶段同量级，说明优化保持稳定；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-25 | 04b73f5d | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,524 / 6,854 / 1,599 ms；39 / 2,653 ms | 4,524 / 6,854 / 1,599 ms；44 / 4,396 ms | 0% | - | release；变化集后处理与默认 ffprobe 并发 64；扫描期间前台 p95 194 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 约 4.4 s，仍高于 500 ms 目标；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-25 | 33fe9db4 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,360 / 5,776 / 1,402 ms；40 / 2,643 ms | 4,360 / 5,776 / 1,402 ms；45 / 4,457 ms | 0% | - | release；fingerprint 命中时跳过逐文件索引修复查询；ffprobe 默认配置 128，扫描期间前台 p95 166 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 约 4.5 s，仍高于 500 ms 目标；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-25 | 4b0561b2 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,254 / 5,502 / 1,419 ms；47 / 2,653 ms | 4,254 / 5,502 / 1,419 ms；52 / 4,397 ms | 0% | - | release；已有文件 fingerprint/stat 使用最多 64 路有界 I/O 并发；ffprobe 默认配置 128，扫描期间前台 p95 170 ms，`foregroundErrors=0`；`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 约 4.4 s，仍高于 500 ms 目标；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |

| 2026-08-25 | 2f4bf2cf | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `cargo test --release --locked --test performance lux_045_catalog_scan_benchmark -- --ignored --nocapture --test-threads=1`（fixture 由 `tools/catalog-fixture/generate.py` 生成） | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,489 / 1,301 / 1,444 ms；40 / 217 ms | 4,489 / 1,301 / 1,444 ms；50 / 322 ms | 0% | - | release；同一用户、权限范围、查询和分页的在途搜索请求使用 singleflight；ffprobe 配额为默认 256、硬上限 512；扫描期间前台 p95 183 ms，`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 已低于 500 ms；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-25 | 80aacea3 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 同上 | 同上 | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,730 / 1,420 / 1,537 ms；46 / 284 ms | 4,730 / 1,420 / 1,537 ms；53 / 394 ms | 0% | - | release；补充失败 search flight 唤醒修复；singleflight、ffprobe 256 默认/512 硬上限保持；扫描期间前台 p95 193 ms，`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 仍低于 500 ms；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-25 | cf8a567a | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 4,651 / 1,368 / 1,519 ms；46 / 302 ms | 4,651 / 1,368 / 1,519 ms；54 / 412 ms | 0% | - | release；完整 LUX-045/LUX-197 脚本；singleflight 失败唤醒修复、ffprobe 256 默认/512 硬上限；扫描期间前台 p95 196 ms，`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；搜索 p95 低于 500 ms；仅代表本机 ARM64，不外推 NAS/x86_64 性能 |
| 2026-08-29 | 20e102f0 | macOS ARM64 (`aarch64-apple-darwin`, `uname -m=arm64`) | 确定性 60,000 MKV / 600 目录 | `./scripts/run-performance.sh` | 首次扫描 / 无变化重扫 / 单目录增量；目录列表 / 搜索 | 6,128 / 1,511 / 22 ms；79 / 221 ms | 6,128 / 1,511 / 22 ms；85 / 321 ms | 0% | - | release；电影目录按批次读取已有索引、64 路有界 fingerprint 检查、并发准备新文件并批量事务写入；剧集无变化 fingerprint 检查并发执行，provider ID 回写串行去重；扫描期间前台 p95 241 ms，`foregroundErrors=0`、`metadataFingerprintCount=0`、`nonPendingProbeCount=0`；仅代表本机 ARM64，不外推 NAS/x86_64 |

## LUX-197 ffprobe 并发记录

ffprobe 合成基准包含 512 个文件，`observed` 是 fake ffprobe 进程的最大重叠数。资源背压会根据本机 CPU、内存
和前台压力把实际值压低，因此 `requested` 是配置值，不是强制启动数。fake ffprobe 使用单进程 Python helper，
只用文件锁保护计数，不额外派生 sleep 子进程，避免测试工具自身放大高并发压力。

| 日期 | 提交 | 架构 | 请求并发 | 实测最大并发 | 耗时 | 命令 |
|---|---|---|---:|---:|---:|---|
| 2026-08-25 | cf8a567a | macOS ARM64 (`uname -m=arm64`) | 128 | 49 | 3,506 ms | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` |
| 2026-08-25 | cf8a567a | macOS ARM64 (`uname -m=arm64`) | 256 | 91 | 3,192 ms | 同上 |
| 2026-08-25 | cf8a567a | macOS ARM64 (`uname -m=arm64`) | 384 | 72 | 3,115 ms | 同上 |
| 2026-08-25 | cf8a567a | macOS ARM64 (`uname -m=arm64`) | 512 | 75 | 3,120 ms | 同上 |
| 2026-08-25 | 345c6d3a | macOS ARM64 (`uname -m=arm64`) | 128 | 62 | 3,191 ms | `cargo test --release --locked --test performance lux_197_ffprobe_concurrency_benchmark -- --ignored --nocapture --test-threads=1` |
| 2026-08-25 | 345c6d3a | macOS ARM64 (`uname -m=arm64`) | 256 | 68 | 2,898 ms | 同上 |
| 2026-08-25 | 345c6d3a | macOS ARM64 (`uname -m=arm64`) | 384 | 63 | 2,917 ms | 同上 |
| 2026-08-25 | 345c6d3a | macOS ARM64 (`uname -m=arm64`) | 512 | 89 | 2,913 ms | 同上 |
| 2026-08-25 | 4b0561b2 | macOS ARM64 (`uname -m=arm64`) | 64 | 45 | 20,512 ms | `LUX_PERF_FILE_COUNT=60000 ./scripts/run-performance.sh` |
| 2026-08-25 | 4b0561b2 | macOS ARM64 (`uname -m=arm64`) | 128 | 69 | 35,961 ms | 同上 |
| 2026-08-25 | 4b0561b2 | macOS ARM64 (`uname -m=arm64`) | 192 | 82 | 41,628 ms | 同上 |
| 2026-08-25 | 4b0561b2 | macOS ARM64 (`uname -m=arm64`) | 256 | 89 | 41,898 ms | 同上 |

这组结果证明 512 路配置可被接受且全局 semaphore 没有超过硬上限；当前开发机观察值受动态背压和进程启动开销影响，
不能据此声称目标 NAS 的实际吞吐。ffprobe 默认配置为 256；4/8/16 核环境的正常有效目标分别为 128/256/512，压力升高时会降档。本次 512 个源在四档请求下均成功完成。

## Web 首屏资源记录

| 日期 | 提交 | 硬件/架构 | 数据集 | 命令 | 指标 | 优化前 | 优化后 | 备注 |
|---|---|---|---|---|---|---:|---:|---|
| 2026-08-11 | 899c961a / 65311847 | macOS ARM64 (`uname -m=arm64`) | Web production build；不含媒体库数据 | `pnpm --dir web build` | 主 JS（原始 / gzip） | 661.09 / 194.28 kB | 493.90 / 153.15 kB | 路由按需加载、首页 logo 复用已有标签；gzip 体积下降约 21%；未测量浏览器 LCP 或首页 API p95 |
| 2026-08-31 | `ca737f79` / `170e41a5` | macOS ARM64 (`uname -m=arm64`) | Web production build；不含媒体库数据 | `pnpm --dir web build` | 主入口 / PlayerPage / router / HLS（原始 / gzip） | 99.54 / 26.22；78.78 / 25.14；0 / 0；594.13 / 185.60 kB | 99.72 / 26.25；79.07 / 25.26；38.71 / 14.02；594.13 / 185.60 kB | 时间线 React 更新限制为 100 ms；bootstrap/session 请求可取消；react-router 共享分包从空 chunk 修正为可复用 chunk；HLS 仍仅在服务端 HLS 路径动态加载；未测量真实浏览器 LCP，结果仅代表本机 ARM64 |

## Web 客户端 HEVC fallback 性能

这些结果只表示本机客户端处理能力，不代表目标 x86_64 NAS 性能。`speedX` 定义为媒体时长除以 Worker 的
解码/编码处理耗时；小于 1 表示客户端转码本身慢于实时播放。

| 日期 | 提交 | 硬件/浏览器 | 样本 | 命令/场景 | 媒体时长 | Worker 处理 | speedX | 丢帧/同步 |
|---|---|---|---|---|---:|---:|---:|---|
| 2026-08-17 | `fa39190a` | macOS arm64 / HeadlessChrome 151 | 3840×2160 HEVC Main 8-bit + AAC、MP4 | Playwright `ClientHevcEngine.setSource` + 播放 2 秒 + seek | 8,000 ms | 21,558.7 ms | 0.371 | 50 帧/0 丢帧；播放漂移 30 ms，seek 漂移 36 ms |
| 2026-08-17 | `fa39190a` | macOS arm64 / HeadlessChrome 151 | 3840×2160 HEVC Main10 10-bit、MP4、无音频 | 同上 | 4,086 ms | 18,929.3 ms | 0.216 | 24 帧/0 丢帧；seek 通过 |

流式播放增量 `43a7b8e6` 复测如下；`setSource()` 在首个视频片段进入 MSE 后返回，完整输入读取、解码、编码和 `endOfStream` 在后台继续。`presentedFrameGaps` 由 `requestVideoFrameCallback` 的 `presentedFrames` 序列计算；HeadlessChrome 的 `getVideoPlaybackQuality().droppedVideoFrames` 累计值与实际 presented-frame 序列不一致，因此不作为本次丢帧结论。

| 日期 | 提交 | 硬件/浏览器 | 样本 | 命令/场景 | 媒体时长 | Worker 处理 | speedX | 丢帧/同步 |
|---|---|---|---|---|---:|---:|---:|---|
| 2026-08-17 | `43a7b8e6` | macOS arm64 / HeadlessChrome 151 | 3840×2160 HEVC Main 8-bit + AAC、MP4 | Playwright 流式 `setSource` + 首段播放 + 完整转码 + seek | 8,000 ms | 17,383.5 ms | 0.460 | 47 个 presented frame callback、0 个 frame gap；首段返回 4,537 ms，完整 17,665 ms；seek 87 ms，音画差约 44 ms |
| 2026-08-17 | `43a7b8e6` | macOS arm64 / HeadlessChrome 151 | 3840×2160 HEVC Main10 10-bit HDR10、MP4、无音频 | 同上 | 4,086 ms | 18,227.5 ms | 0.224 | 4 个 presented frame callback、0 个 frame gap；首段返回 9,606 ms，完整 18,577 ms；seek 79 ms |

4K 两条记录均未通过实时性能门；播放器已把该状态暴露给用户，并建议原生客户端或降低清晰度。样本 SHA-256
和完整兼容性结论见 `docs/COMPATIBILITY.md`。

## 首页加载基线

| 日期 | 提交 | 硬件/架构 | 数据集 | 命令/场景 | p50 | p90 | p95 | 最大值 | 备注 |
|---|---|---|---|---|---:|---:|---:|---:|---|
| 2026-08-14 | 57bf1b11 | macOS ARM64 (`uname -m=arm64`) | 1,200 个合成空 `.mkv`；单个电影库；无真实图片 | 预热后串行请求 `GET /api/v1/home` 50 次 | 2.411 ms | 3.595 ms | 4.196 ms | 9.111 ms | 本机服务；浏览器首页 API 约 4–6 ms，渲染 12 张媒体卡片，未发现 long task；该数据不代表目标 x86_64 NAS，也不能证明真实图片负载已达标 |
| 2026-08-14 | a812afe4 | macOS ARM64 (`uname -m=arm64`) | 同上 | release 服务；预热后串行请求 `GET /api/v1/home` 50 次 | 2.375 ms | 2.526 ms | 2.615 ms | 3.430 ms | 后端聚合优化后；ACL 只取一次、首页区块复用库 ID、用户状态跨区块去重批量查询；仅复测 API，未重新测量浏览器 LCP；该数据不代表目标 x86_64 NAS |
| 2026-08-14 | 633bfe4f | macOS ARM64 (`uname -m=arm64`) | 同上 | 干净提交的 release 服务；预热后串行请求 `GET /api/v1/home` 50 次 | 2.384 ms | 2.622 ms | 2.658 ms | 3.876 ms | 独立复核；与上一条结果同量级；不代表目标 x86_64 NAS |

浏览器复核（633bfe4f，空媒体库）：测试账户登录后首页正常渲染；页面隐藏状态模拟 20 秒期间 `/api/v1/home` 请求数没有增加，恢复可见后约 2.5 秒内增加 1 次刷新。测试账户没有头像，因此控制台只有预期的头像 404；未以该空媒体库结果宣称真实图片 LCP 达标。

### 推荐计算专项记录

| 日期 | 提交/工作树 | 架构 | 数据集 | 场景 | 结果 | 备注 |
|---|---|---|---|---|---|---|
| 2026-08-31 | 优化前基线 | macOS ARM64（`uname -m=arm64`） | 约 60,000 条媒体、约 65,000 条用户状态 | 评分中位数、推荐主查询、冷缓存完整推荐 | 约 53 ms、91 ms、144 ms | 中位数排序只在冷缓存执行；主成本是播放用户去重和分组临时 B-tree；不能外推 NAS/x86_64 |
| 2026-08-31 | 工作树 | macOS ARM64（`uname -m=arm64`） | 同上 | 冷启动完整推荐 / 同批次后续推荐 | 201.8 ms / 0.75 ms | 冷启动包含一次 180 天播放去重、收藏聚合和评分中位数；后续请求读取每日推荐 ID 和物化统计；本机 ARM64 结果不能外推 NAS/x86_64 |

## 元数据刮削请求计数验证

| 日期 | 提交 | 硬件/架构 | 数据集/命令 | 场景 | 优化前 | 优化后 | 备注 |
|---|---|---|---|---|---:|---:|---|
| 2026-08-19 | `e447f24` | macOS ARM64 (`uname -m=arm64`) | 两个 TMDb 搜索候选；`cargo test --locked --test metadata_selection automatic_candidate_search_expands_only_the_best_result` | 自动匹配候选展开 | 2 个候选都完整请求详情、图片、演职员等 | 1 个候选完整请求 + 1 个搜索摘要 | 集成测试确认第二候选没有详情请求；这是请求计数验证，不代表真实 TMDb/NAS 延迟 |
| 2026-08-19 | `7350f68` | macOS ARM64 (`uname -m=arm64`) | TMDb stub；`cargo test --locked --test tmdb tmdb_client_coalesces_and_reuses_cached_requests` | 同一搜索请求连续执行两次 | 2 次上游请求 | 1 次上游请求 | 证明进程缓存命中；缓存文件恢复和 singleflight 另有单元测试 |

这里的 p90/p95 是请求耗时分布的位置：例如 p95=4.196 ms 表示 50 次请求中约 95% 不超过 4.196 ms，剩余约 5% 更慢；它们用于观察尾部延迟，不是平均值。由于本次样本只有 50 次，百分位数仅作开发机基线，不能替代目标数据集上的正式验收。

### LUX-200 阶段指标与回归验证

LUX-200 的后台元数据指标通过管理员健康资源接口中的 `resources.metadata` 暴露。计数器只使用固定低基数标签：
`search`、`bundle`、`get`、`images`、`credits`、`external_ids`、`trailers`，以及
`queue_wait`、`item_total`、`image_download`、`image_write`、`cache_persist`、`nfo_write` 阶段；不会包含用户 ID、完整 URL、token 或原始错误文本。
`stageP95Ms` 使用有界的最近样本窗口。缓存和 singleflight 分别记录 `cache.hit.count` 与 `cache.miss.count`，刮削器重试记录对应 capability 的 `retry.*.count`，图片累计字节记录在 `image.bytes`。
缓存落盘另记录 `cache.persist.success.count`、`cache.persist.error.count` 和 `stageP95Ms.cache_persist`，用于区分缓存命中收益与落盘背压。

| 日期 | 提交 | 验证 | 结果 | 限制 |
|---|---|---|---|---|
| 2026-08-26 | 工作树（`uname -m=arm64`） | `cargo test --locked --test metadata_selection fill_missing_only_requests_the_missing_image_capability` | 只缺 poster 时仅命中 `/3/movie/1/images`；补齐 poster 后第二次 `FILL_MISSING` 上游请求数为 0 | 本地 TMDb stub，非真实 TMDb/NAS 延迟 |
| 2026-08-26 | 工作树（`uname -m=arm64`） | `cargo test --locked --test image_writer image_downloads_respect_the_global_concurrency_limit` | 6 个并发图片写入在测试 semaphore=2 时最大并发不超过 2 | 证明配额边界，不代表上游吞吐 |
| 2026-08-27 | `8ab96ce7`（`uname -m=arm64`） | `cargo test --locked --test reidentify fill_missing_skips_complete_movie_without_scraper_request` | 完整电影 `FILL_MISSING` 上游请求数为 0；删海报后补全会重新产生请求 | 完整夹具包含 NFO rich details、人物关系和多 provider ID；本地 TMDb stub |
| 2026-08-27 | `de7aad98`、`118260b7`（`uname -m=arm64`） | `cargo test --locked --lib application::images::tests::permanent_upstream_status_does_not_schedule_image_retry`；`cargo test --locked --test image_writer successful_image_retry_clears_the_backoff_state` | 403 不安排 `next_retry_at`；临时失败到期后的成功下载将状态置为 `AVAILABLE` 并清除退避 | 状态机回归验证，不代表真实上游延迟或吞吐 |
| 2026-08-27 | `1eb460d2`（`uname -m=arm64`） | `./scripts/run-metadata-performance.sh`（连续 5 次） | 每次 32/32 条目成功；吞吐 30.9–37.0 条/秒；每次 32 次 search、32 次 bundle；图片 28 条可用、4 条明确不可用、1 次临时重试；代表性一次 `elapsed=918ms`、`stageP95Ms={bundle:4,image_download:0,image_write:78,item_total:469,nfo_write:147,queue_wait:31,search:3}`、`imageBytes=1876` | SQLite 最终元数据选择事务使用 `BEGIN IMMEDIATE`；修复前并发基准偶发 `SQLITE_BUSY`/`SQLITE_BUSY_SNAPSHOT`；仅代表本机 ARM64，不外推 NAS/x86_64 |
| 2026-08-27 | `1be3f59e`（`uname -m=arm64`） | `./scripts/run-metadata-performance.sh`（release，单次复测） | 32/32 条目成功；`elapsed=772ms`、吞吐 41.4 条/秒；32 次 search、32 次 bundle；图片 28 条可用、4 条明确不可用、1 次临时重试；`stageP95Ms={bundle:3,image_download:0,image_write:44,item_total:215,nfo_write:60,queue_wait:18,search:3}`；`imageBytes=1876` | 本次拆分下载/写入配额后未见基准退化；该 benchmark 使用 adapter stub，不触发持久化 provider cache，`cache_persist` 由独立指标测试覆盖；仅代表本机 ARM64，不外推 NAS/x86_64 |
| 2026-08-27 | `00b7a472`（`uname -m=arm64`） | `./scripts/run-metadata-performance.sh`（release，连续 5 次） | 32/32 条目均成功；耗时 849–895 ms，吞吐 35.7–37.7 条/秒；每次 32 次 search、32 次 bundle；图片每次 28 条可用、4 条明确不可用、1 次临时重试；`stageP95Ms` 代表性范围为 `bundle:3–4,image_download:0,image_write:27–33,item_total:119–131,nfo_write:32–37,queue_wait:6–11,search:3–4`；`imageBytes=1876` | SQLite 默认 4 路元数据 worker，进程级硬上限 16；本机 ARM64，不能外推 NAS/x86_64；adapter stub 不触发持久化 provider cache |
| 2026-08-27 | `1c1c52e9`（`uname -m=arm64`） | `./scripts/run-metadata-performance.sh`（release，单次最终复测） | 32/32 条目成功；`elapsed=875ms`、吞吐 36.6 条/秒；32 次 search、32 次 bundle；图片 28 条可用、4 条明确不可用、1 次临时重试；`stageP95Ms={bundle:4,image_download:0,image_write:30,item_total:114,nfo_write:34,queue_wait:7,search:4}`；`imageBytes=1876` | 最终并发/压力降档实现复测；结果与前一组连续 5 次基准同量级；adapter stub 不触发持久化 provider cache；仅代表本机 ARM64，不外推 NAS/x86_64 |
| 2026-08-29 | `7b76f3bd`（`uname -m=arm64`） | `./scripts/run-metadata-performance.sh`（release，连续 3 次） | `FILL_MISSING` 候选无 credits 时跳过重复 `people.json`/人物关系索引写回；32/32 条目成功；耗时 525–598ms，吞吐 53.4–60.8 条/秒；每次 32 次 search、32 次 bundle；图片 28 条可用、4 条明确不可用、1 次临时重试；代表性 `stageP95Ms={bundle:3,image_download:0,image_write:29,item_total:78,nfo_write:28,queue_wait:4,search:3}`；`imageBytes=1876` | 相比此前 849–895ms 基线，提升受本机 I/O/调度噪声影响；仅代表本机 ARM64，不外推 NAS/x86_64；adapter stub 不触发持久化 provider cache |
| 2026-08-27 | `00b7a472`（`uname -m=arm64`） | `cargo test --locked --test postgres_database -- --ignored --nocapture`（临时 `postgres:16-alpine`） | PostgreSQL 空库迁移、核心状态、元数据优先级/锁定字段/图片/人物关系、重扫布尔投影和 STRM 配置共 4/4 通过 | 临时本地容器，测试完成后已删除；不代表生产 NAS 连接池或远程磁盘延迟 |

本机架构需以 `uname -m` 记录；ARM64 测试结果不能外推到目标 NAS/x86_64。

## ARM 开发机检查

- 架构：后续记录 `uname -m` 输出（当前为 `arm64`）。
- 用途：验证本机编译、单元/集成测试和工具链行为。
- 限制：不得将本机 ARM 结果当作目标 x86_64 NAS 的正式性能报告。

## LUX-045 ARM64 结果说明

- 固定入口：`scripts/run-performance.sh`；脚本临时生成 fixture，测试完成后删除，不提交 60,000 个媒体文件。
- fixture manifest：`lux-catalog-fixture-v1`，60,000 个文件、600 个目录、固定内容摘要 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。
- 结果证明扫描期间前台请求没有出现错误或长时间锁等待；这只是本机 ARM64 基线，不代表 NAS/x86_64 容量结论。
- 无变化重扫的扫描路径只执行 fingerprint 检查；性能测试确认 `probe_status` 仍为 `PENDING` 且 `metadata_fingerprint` 仍为空。
- 2026-08-03 的新结果用于当前提交 `50a9e09` 的阶段性回归；首次扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-03 的新结果用于当前提交 `c23a757`；批量用户状态查询已消除 Web/Emby 列表的逐条状态读取，但本次扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-03 的新结果用于当前提交 `df28a97`；启动恢复逻辑未改变扫描基准的访问模式，首次扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-03 的新结果用于当前提交 `8796365`；健康诊断和 reconcile 路由不改变扫描基准的访问模式，首次/无变化扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-03 的新结果用于当前提交 `ba39b1d`；媒体 root 恢复和磁盘故障烟测不改变基准的访问模式，首次/无变化扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-03 的新结果用于当前提交 `b42a133`；扫描后 ffprobe 接入只在后台 job 完成后执行，基准直接调用 `LibraryScanner`，本次仍确认扫描期间前台 p95 12 ms、无错误，首次/无变化扫描耗时受本机负载影响，不能与上一条结果直接视为性能退化结论。
- 2026-08-08 的新结果用于当前提交 `f3f0d460`；媒体可用性改为物化字段并由触发器维护，电影首扫新增文件采用批量事务，搜索结果和详情采用批量加载，FTS 命中时跳过全表 LIKE 分支；新增目录列表和搜索并发指标，结果仍仅代表本机 ARM64。
- 2026-08-09 的新结果用于当前提交 `c022fcac`；新增电影后台任务的有界文件准备并发、容器 CPU 配额和首页 p95 自适应降档、按根批量写入；基准脚本本身仍是直接扫描路径，不能据此宣称持久化后台任务的精确耗时变化。
- 2026-08-10 的新结果用于提交 `5e0bef61`；目录聚合请求使用有界背压，50 个并发目录请求全部成功。剧集、合集、Resume、STRM 与弹幕的大数据量回归由对应合成数据库测试覆盖；本机没有用户的真实媒体库，Docker daemon 也未运行，因此该记录不证明目标 NAS 上的峰值 RSS 或任务结束后的 glibc RSS 回收效果。

## LUX-266 Manifest 发现写入边界

- 新建全量扫描时，scan job、Manifest、root 状态和根目录 frontier 在同一短事务中创建；扫描目录只从 Manifest frontier 取出，不再把目录待办写入 `reconciliation_scan_entries`。
- 每个有界发现 chunk 在同一事务中追加不可变 observation、插入子目录 frontier、更新 Manifest/root/job 计数，并只在成功枚举目录的最终 chunk 标记该目录完成。取消或失败保留已提交 observation/frontier；未完成的 root 标记为 `INCOMPLETE`，不可进入后续缺失判断。
- 每条 observation 使用 11 个绑定参数，按 80 条/语句（最多 880 binds）写入；子目录按 200 条（600 binds）写入；为保持本任务增量独立，已发现文件暂由旧文件索引工作队列承接，按 200 条（最多 800 binds）写入。LUX-267 完成 Manifest delta apply 后再移除这段过渡桥接。
- `cargo test --locked --test scanning_jobs` 覆盖 1,025 个文件的跨批次发现、observation 指纹/重观察版本、取消时保留已提交 frontier，以及 root 不可用后恢复。该测试证明正确性和 SQLite 批次边界，不是耗时/吞吐基准；此任务未运行 release benchmark，也不据此声称扫描速度提升或推断 PostgreSQL/NAS 性能。

## LUX-270 Manifest SQLite/PostgreSQL 阶段门

2026-09-24 在本机 ARM64（`uname -m=arm64`，Rust `aarch64`）使用相同的固定 fixture 运行 release 基准。fixture 为 60,000 个文件、600 个目录，SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。基准二进制报告的基线提交为 `4802c939`，测量包含其后的 LUX-270 工作树改动；数据仅用于同一 ARM64 开发机对照。

| 后端 | Manifest 首扫 | batches / p50 / p95 | SQL / DML | 无变化重扫 | 50 并发管理请求 p95 / 目录列表 p95 | 锁 / WAL |
|---|---:|---|---:|---:|---:|---|
| SQLite | 15.796 s | 640 / 23 ms / 28 ms | 30,581 / 13,870 | 1.289 s（41 batches） | 241 ms / 337 ms | `busy_timeout=5000 ms`；154 次 `BEGIN IMMEDIATE` admission 样本：p50 21 µs、p95 10,058 µs、max 10,113 µs、0 次错误 |
| PostgreSQL 16（本地临时容器） | 142.229 s | 640 / 82 ms / 505 ms | 32,436 / 13,870 | 10.589 s（41 batches） | 272 ms / 641 ms | 写入 WAL 596,331,453 bytes；5,197 次锁等待采样，观察到的最大 waiter 数为 0 |

两组均处理 60,000 个新文件；PostgreSQL 数据库为该次测试专用空库。执行命令：

```bash
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
scripts/run-performance.sh

LUX_PERF_BACKEND=postgres \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
POSTGRES_TEST_HOST=127.0.0.1 \
POSTGRES_TEST_PORT=55432 \
POSTGRES_TEST_DATABASE=your_disposable_empty_database \
POSTGRES_TEST_USER=your_test_user \
scripts/run-performance.sh
```

同日以相同 fixture 在提交 `4802c939` 重跑 SQLite LUX-045 直接扫描：2.105 s、3,254 SQL、2,404 DML；LUX-270 最初的 SQLite Manifest 实现为 202.468 s、506,457 SQL、310,870 DML。此次 Manifest 批量 CAS/Delta 更新与索引化最新观察分页后，相比最初 SQLite Manifest 版本首扫约快 12.8 倍，SQL 约减少 16.6 倍，DML 约减少 22.4 倍。Manifest 首扫仍比直接扫描基线慢；两条路径的持久化与安全语义不同，不能将它们当作同一工作量下的等价耗时。PostgreSQL 结果仅为本机临时容器单次观测，不能将 ARM64 数值外推至 NAS/x86_64。SQLite 锁 admission canary 会每 100 ms 尝试一次 `BEGIN IMMEDIATE` 并立即提交，采样本身可能轻微扰动扫描；PostgreSQL 锁采样通过 `pg_stat_activity` 读取，SQL 计数中排除了这些监控查询。

### Manifest 首扫优化复测

2026-09-24 在本机 ARM64（`uname -m=arm64`）对同一 60,000 文件 / 600 目录 fixture 连续运行三次 SQLite release 基准，关闭写锁采样器以减少测量扰动。基准二进制显示提交 `ac869321`，扫描代码为其上的工作树修改。

| 场景 | 三次结果 | 中位数 | SQL / DML | 备注 |
|---|---:|---:|---:|---|
| Manifest 首扫 | 7.682 / 7.707 / 7.705 s | **7.705 s** | 10,343 / 5,418 | 120 个 500-delta apply 批次；小目录发现合并为 76 个事务；apply 中位数：应用侧 2.569 s、事务 3.786 s |
| Manifest 无变化重扫 | 1.254 / 1.251 / 1.215 s | **1.251 s** | — | 41 个批次，无索引 apply |
| 扫描期间前台 50 请求 | p95 231 / 268 / 234 ms | **234 ms** | — | 目录列表 p95 中位数 419 ms |
| 旧直接扫描器 | 1.994 s | — | 3,254 / 2,404 | 同机、同 fixture 的一次基准；不是与完整 Manifest 任务相同的持久化/恢复工作量 |

本轮对比 Manifest 初版 15.796 s，首扫中位数减少约 51.2%；SQL/DML 从 30,621/13,870 降至 10,343/5,418。与旧直接扫描 1.994 s 相比仍慢约 **3.9 倍**，**未通过“首扫不能比原版慢”的目标**。已测优化包括 500 条有界 apply 事务（SQL 仍按 SQLite 参数安全上限分块）、复用持久化 observation、按扫描并发并行准备且只保留一次最终设备/inode/fingerprint 复核、合并小目录发现 checkpoint、合并差异页事务、独立 500-path target SQL 块，以及按层级批量插入新电影父目录。三次复测中 apply 阶段仍占约 6.5 s，是下一轮主要优化对象。

三次观测有约 1 秒波动，故记录中位数；这些数字只代表本机 ARM64 与 SQLite，不外推 NAS/x86_64 或 PostgreSQL。LUX-267 v2 将以发现事务作为正向索引 checkpoint，避免为每个 ADD/CHANGE 持久化并二次应用 delta；只有完整根路径上的 REMOVE 仍走持久化 delta 与二次确认。当前旧直接扫描 1.994 s 是同 fixture 的性能参照，不代表相同持久化合同；重构目标是在保留不可变 observation、CAS、删除确认、取消恢复和原子 checkpoint 的前提下尽量逼近该参照。新实现须用同一 ARM64/SQLite fixture 三次 release 中位数测量，并另行运行 PostgreSQL 阶段门。

### LUX-267 v2 正向索引与 Manifest 写入复测

2026-09-25 在同一 ARM64（`uname -m=arm64`，Rust 1.97.1）与 SQLite release 环境，对 60,000 个 MKV / 600 个目录 fixture 连续测三次；SHA-256 为 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。基准显示代码提交 `ac869321`，包括当前未提交的 LUX-267 修改；关闭 SQLite 写锁采样器。

| 场景 | 三次结果 | 中位数 | SQL / DML | 备注 |
|---|---:|---:|---:|---|
| Manifest 首扫 | 3.252 / 2.836 / 2.853 s | **2.853 s** | 4,383 / 2,953 | 13 个外层批次、66 个正向索引事务；正向提交阶段 1.897 / 1.862 / 1.845 s |
| Manifest 无变化重扫 | 0.966 / 0.959 / 0.968 s | **0.966 s** | — | 13 个批次 |
| 扫描期间前台 50 请求 | p95 236 / 227 / 228 ms | **228 ms** | — | 目录列表 p95 中位数 346 ms |

当前代码较 2026-09-24 的前一组 Manifest 复测中位数 3.100 s 快约 8%；受单次数据波动影响，不将差值全部归因于代码优化。v2 避免为正向文件构造不会持久化的 delta 对象；0134 另删除与主键列序完全相同的 `idx_scan_manifest_entries_path`。SQL/DML 数量未因此变化，表明本机测量没有清晰分离出该索引带来的耗时收益。旧直接扫描 1.994 s 仍只是较早提交的参照；本轮试跑该旧入口超过 4 分钟仍未完成且未输出首扫结果，已中止，因此没有当前版本的同代码对照。本记录不宣称已达到“不慢于原版”的目标，也不外推 PostgreSQL 或 NAS 性能。

### LUX-267 discovery format 3 混合 presence 复测

2026-09-25 在相同 ARM64/SQLite release 环境与 60,000 MKV / 600 目录 fixture 上运行三次；fixture SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`，代码提交 `ac869321` 加工作树修改，关闭 SQLite 锁采样器。

| 场景 | 三次结果 | 中位数 | SQL / DML | 备注 |
|---|---:|---:|---:|---|
| SQLite format 3 首扫 | 3.069 / 2.618 / 2.633 s | **2.633 s** | 3,727 / 2,475 | 66 个 1,000-path 正向事务；format=3、seen-path=0、完整 FILE observation=0、generation 标记 60,000 条 |
| SQLite format 3 无变化重扫 | 0.936 / 0.956 / 0.960 s | **0.956 s** | — | 13 个批次；seen-path ledger 记录 60,000 条未变化路径，generation 未被重写 |
| SQLite 扫描期间前台 50 请求 | p95 234 / 230 / 229 ms | **230 ms** | — | 目录列表 p95 中位数 364 ms |
| PostgreSQL 16 首扫（本机 ARM64 Docker） | 13.180 / 12.617 / 14.148 s | **13.180 s** | 3,802 / 2,475 | 66 个 1,000-path 事务；WAL 297,865,996 / 311,672,541 / 314,576,597 bytes |
| PostgreSQL 16 无变化重扫 | 4.032 / 4.015 / 4.040 s | **4.032 s** | — | 13 个批次；锁等待采样关闭 |
| PostgreSQL 16 扫描期间前台 50 请求 | p95 258 / 266 / 259 ms | **259 ms** | — | 目录列表 p95 中位数 635 ms |

与前一组 format 2 首扫中位数 2.853 s 相比，SQLite format 3 快约 **7.7%**，SQL/DML 分别减少约 15.0%/16.2%；无变化重扫由 0.966 s 到 0.956 s。成功正向索引由当前 filesystem generation 标记，因此新库首扫不写 seen-path；未变化、unstable 或 CAS 未成功路径才写 ledger。2,000-path 事务试验首扫中位数 2.653 s，慢于最终保留的 1,000-path 结果，故仍使用 1,000。

PostgreSQL 三次使用不同临时空库，format 3 首扫中位数 13.180 s、无变化重扫 4.032 s；这验证了本机 PostgreSQL 16 Docker 上的真实 migration、写入和删除合同。PG 锁等待采样关闭。历史旧直接扫描 1.994 s 来自较早提交；当前 ARM64/SQLite format 3 的 2.633 s 仍比该参照慢约 32%，本机旧入口超过 4 分钟未完成，未获得当前代码的直接对照。该差异和本机 Docker PG 数字都不外推 NAS/x86_64 或生产挂载盘。

### LUX-267 v3 target checkpoint 与双后端复测

2026-09-25 在相同 ARM64 环境、Rust 1.97.1、60,000 MKV / 600 目录 fixture（SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`）上，对加入 0136 target checkpoint 的工作树进行 release 基准。SQLite 连测五轮并关闭锁采样；PostgreSQL 16.15/aarch64 Docker 连测三轮，每轮使用新建的空数据库并采集锁等待和 WAL。索引耗时到 `POSTPROCESSING`；target 物化单独计时，处理 120,000 个 SOURCE/ITEM target。

| 后端 | 索引完成：各轮 / 中位数 | target 物化：各轮 / 中位数 | 无变化重扫：各轮 / 中位数 | SQL / DML | 前台 50 请求 p95 中位数 | WAL / 锁 |
|---|---:|---:|---:|---:|---:|---|
| SQLite | 2.058 / 2.111 / 2.018 / 2.008 / 1.950 s；**2.018 s** | 556 / 531 / 530 / 526 / 546 ms；**531 ms** | 925 / 941 / 936 / 917 / 915 ms；**925 ms** | 673 / 209 | 234 ms | `synchronous=FULL`；锁采样关闭 |
| PostgreSQL 16 | 8.703 / 9.657 / 10.591 s；**9.657 s** | 2.442 / 2.503 / 2.344 s；**2.442 s** | 3.641 / 5.031 / 3.743 s；**3.743 s** | 692 / 209 | 257 ms | WAL 215,655,557 / 217,383,695 / 219,460,717 bytes；三轮最大 waiter 均为 0 |

两种后端均使用 13 个扫描批次；PostgreSQL 有 10 个正向提交批次，正向提交中位数 7.268 s，准备中位数 607 ms。SQLite 正向提交中位数 1.150 s。PostgreSQL 目录列表 p95 中位数为 605 ms。性能 harness 的 target 计数/ready 查询已改为后端对应的 bind 占位符；此前 PostgreSQL 首轮基准因此报语法错误，该轮不纳入性能样本。

复测命令：

```bash
LUX_PERF_DISABLE_LOCK_MONITOR=1 \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_DIRECTORY_COUNT=600 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
scripts/run-performance.sh

LUX_PERF_BACKEND=postgres \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_DIRECTORY_COUNT=600 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
POSTGRES_TEST_HOST=127.0.0.1 \
POSTGRES_TEST_PORT=55432 \
POSTGRES_TEST_DATABASE=lux_perf_run_1 \
POSTGRES_TEST_USER=lux \
scripts/run-performance.sh
```

历史直接扫描器的 1.994 s 是较早提交的单次参照；本轮 SQLite Manifest 中位数高 24 ms（约 1.2%），且本轮样本范围为 1.950–2.111 s。两条路径的恢复和持久化工作不同，当前数据支持“约 2 秒索引完成”的结果，不能证明完整 Manifest 严格快于旧直接扫描。PostgreSQL 数字只代表本机 ARM64 容器，不推断远程数据库、NAS/x86_64 或生产挂载盘。

在 target-page / file-batch 参数整理后，用同一 SQLite fixture 又做三次 release spot check：索引完成 **2.913 / 2.005 / 2.042 s**，target 物化 **562 / 534 / 542 ms**，无变化重扫 **920 / 940 / 949 ms**，SQL/DML 为 **675 / 677 / 675 / 209**，target 数仍为 120,000。中位数分别为 2.042 s、542 ms、940 ms；首轮 2.913 s 是这一组三次的高值，因此保留完整样本供后续复测，不以它替换上面的五轮 SQLite 与三轮 PostgreSQL跨后端比较表。按这组三次 spot-check 中位数与历史 1.994 s 单次旧扫描参照相比，高 48 ms（约 2.4%）；仍不能把不同持久化/恢复语义的单次旧值视为严格同口径验收线。

### LUX-271 v3 资源感知扫描与目录 reader 实验

2026-09-25 在本机 ARM64（`uname -m=arm64`，Rust 1.97.1）使用同一 60,000 文件 / 600 目录 fixture，SHA-256 为 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。基准构建提交为 `3f18aca1` 加工作树改动，关闭锁采样器。最终代码采用单 reader 顺序发现；基准报告的准备并发为 9，目录 reader 并发为 1。SQLite 与 PostgreSQL 均处理 13 个扫描批次、10 个正向提交批次和 120,000 个 postprocessing target。

| 后端 | 索引完成：各轮 / 中位数 | DISCOVERING / 正向准备 / 正向提交中位数 | target 物化中位数 | 无变化重扫中位数 | batch p50 / p95 中位数 | 前台 p95 / 目录列表 p95 中位数 | SQL / DML | WAL 中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite，最终 sequential reader | 2.549 / 2.094 / 2.232 s；**2.232 s** | 2.224 / 0.592 / 1.247 s | 596 ms | 967 ms | 227 / 253 ms | 242 / 350 ms | 675 / 209 | — |
| PostgreSQL 16，最终 sequential reader | 55.841 / 9.078 / 16.646 s；**16.646 s** | 16.629 / 0.623 / 8.777 s | 2.875 s | 3.913 s | 928 / 3,944 ms | 270 / 595 ms | 692 / 209 | 215,871,368 bytes |

`DISCOVERING` 包含目录读取、frontier 和 checkpoint；正向准备与事务写入另有 tracing 计时。两后端的首扫样本波动明显。相较 LUX-270 基线，SQLite 索引中位数从 2.018 s 到 2.232 s、前台 p95 从 234 ms 到 242 ms、无变化重扫从 925 ms 到 967 ms；PostgreSQL 索引中位数从 9.657 s 到 16.646 s、前台 p95 从 257 ms 到 270 ms、无变化重扫从 3.743 s 到 3.913 s。该组数据没有通过 LUX-271 的性能门，也不能据此推断 NAS/x86_64 性能。

曾试过两路目录 reader。早期未限制 live reader 数的三轮中位数为 SQLite 2.114 s、PostgreSQL 9.606 s，但代码可能同时保留 64 个目录 reader，也未覆盖空目录替换后的身份复核，因此不能作为可接受结果。把 reader 数限制为两路并补完安全检查后，PostgreSQL 在新建独立数据库中的首扫观测为 10.511、15.855、55.030、14.632 s，波动过大，无法证明稳定收益。为遵守“没有可重复收益时不保留并发复杂度”的验收要求，最终代码移除了并行目录 reader；数据库写入全程仍为单写者。

LUX-271 的扫描配置优先级和目录替换安全检查已保留；并行目录 I/O 的性能验收未通过，项目尚不能据此关闭阶段 22。以上只代表本机 ARM64 与临时 PostgreSQL 16 容器。

## Web Bilibili 弹幕解析

基准脚本为 `scripts/run-danmaku-performance.mjs`，从指定 Git revision 加载优化前解析器，并与当前工作树在相同 Node 进程中交替执行。输入包含 5,000 条合法弹幕和一个超过 4 MiB 的 ASCII XML；每组 5 批、每批 30 个样本，报告各批 p50/p95 的中位数。

| 日期 | 提交 | 硬件/运行时 | 命令 | 场景 | 优化前 p50/p95 | 优化后 p50/p95 | 结果 |
|---|---|---|---|---|---:|---:|---|
| 2026-08-28 | `85db4549` | macOS ARM64 (`uname -m=arm64`), Node 24.14.1 | `LUX_DANMAKU_BASELINE_REF=85db4549^ node --expose-gc scripts/run-danmaku-performance.mjs` | 5,000 条、2,540,007 bytes 合法 XML | 13.423 / 13.735 ms | 13.197 / 13.565 ms | 解析结果均为 5,000 条 |
| 2026-08-28 | `85db4549` | macOS ARM64 (`uname -m=arm64`), Node 24.14.1 | 同上 | 4,194,311 bytes 超大 ASCII XML 大小检查 | 2.094 / 2.879 ms | 0.002 / 0.002 ms | 均返回 `INPUT_TOO_LARGE` |

该结果只代表本机 ARM64 Node 基准，不外推 NAS/x86_64 或所有浏览器；Chrome 151 本地浏览器实测同一合法夹具 p50/p95 为 8.1/8.6 ms。

## 规则

- 首次扫描、无变化重扫、单目录增量、50 并发短 API 请求、扫描并发前台、4 个 Range 连接和任务恢复都要有独立记录。
- 每次性能优化记录硬件、数据集、命令、提交以及前后结果。
- 记录中的路径、token、真实外部 URL 和用户数据必须脱敏。
- SQL 热查询计划记录见 [`docs/SQL-AUDIT.md`](SQL-AUDIT.md)。
### LUX-272 v3 全量扫描分阶段计时

LUX-272 在 `lux_270_manifest_job_scan_benchmark` 的完整 `ScanJobService` 路径中采集固定阶段名、微秒累计耗时、调用数、p50/p95 和处理单元数。报告的 `manifestIndexMs`、`postprocessingTargetMaterializationMs`、`unchangedRescanMs` 与前台请求 p95 是墙钟指标；`manifestStageTimings`、`targetStageTimings` 和 `unchangedRescanStageTimings` 是分项累计时间。分项可能嵌套或并发重叠，不能相加当作墙钟时间。

发现阶段分为 `directory_open`、`directory_readdir`、`directory_stat`、`directory_batch_total`、`baseline_query`、`positive_classification`、`positive_file_prepare` 和 `positive_file_recheck`。事务阶段分为输入校验、writer admission、Manifest 状态读取、目录 frontier 插入/完成、known-path 查询、root checkpoint、observation 插入、正向索引、presence ledger、Manifest/job 计数 checkpoint、commit 和事务总时间。`activePreparationTasksPeak` 与 `activeDirectoryReadersPeak` 取实测活动任务峰值；预算配置只作为背景值，不代替观测值。

事件只包含固定阶段名、微秒、计数和批次规模，不包含媒体名、路径、用户数据或数据库连接信息。每次 60k 报告必须包含首扫阶段 JSON、target 物化阶段、无变化重扫阶段、扫描墙钟时间、前台 p95、SQL/DML、SQLite 锁等待或 PostgreSQL WAL/锁等待。此阶段只增加诊断，不改变扫描调度或数据库语义。

验证命令（SQLite 与 PostgreSQL 各三轮 60k；每轮 PostgreSQL 使用新建空库）：

```bash
LUX_PERF_DISABLE_LOCK_MONITOR=1 \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
scripts/run-performance.sh

LUX_PERF_BACKEND=postgres \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_TEST_FILTER=lux_270_manifest_job_scan_benchmark \
POSTGRES_TEST_HOST=127.0.0.1 \
POSTGRES_TEST_PORT=55432 \
POSTGRES_TEST_DATABASE=your_disposable_empty_database \
POSTGRES_TEST_USER=your_test_user \
scripts/run-performance.sh
```

2026-09-26 使用 Apple M4 / 16 GiB（`uname -m=arm64`，Rust 1.97.1），release 构建提交 `32b28d6a`。固定 fixture 为 60,000 个文件 / 600 个目录，SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。SQLite 关闭 lock canary；PostgreSQL 16 使用本机一次性容器并启用 `pg_stat_activity` lock sampler。数据只代表本机 ARM64，不外推 NAS/x86_64。

| 后端 | 首扫索引完成：三轮 / 中位数 | 扫描批次；batch p50 / p95 中位数 | 正向准备 / 正向提交墙钟累计中位数 | target 物化 / 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | SQL / DML 中位数 | WAL / 锁采样 |
|---|---:|---:|---:|---:|---:|---:|---|
| SQLite | 2.058 / 2.009 / 2.178 s；**2.058 s** | 13；208 / 264 ms | 523 / 1,208 ms | 546 / 961 ms | 259 / 383 ms | 675 / 209 | canary 关闭 |
| PostgreSQL 16 | 19.059 / 10.472 / 10.368 s；**10.472 s** | 13；835 / 2,036 ms | 411 / 7,365 ms | 2,523 / 5,145 ms | 281 / 685 ms | 694 / 209 | WAL 216,506,975 bytes；385 个锁等待采样，中位轮观察最大 waiter 数为 0 |

下表是三轮每轮阶段累计微秒的中位数。`directory_batch_total` 包括 reader 批次开销，readdir/stat 是其内部拆分；同理事务总时间包含内部 SQL 阶段，不能把这些行相加成墙钟时间。

| 阶段（累计微秒） | SQLite | PostgreSQL 16 |
|---|---:|---:|
| directory open | 39,790 | 66,485 |
| readdir | 11,919 | 19,425 |
| stat | 61,034 | 92,844 |
| directory batch total | 108,250 | 169,312 |
| baseline query | 17,397 | 2,128,641 |
| positive classification | 318,442 | 317,933 |
| positive file preparation | 770,302 | 788,440 |
| positive file recheck | 120,280 | 153,753 |
| positive index apply | 950,342 | 6,458,998 |
| presence ledger | 5,895 | 21,658 |
| transaction begin | 445 | 4,090 |
| transaction commit call | 218,952 | 75,649 |
| transaction total | 1,206,315 | 7,384,819 |

六轮的活动峰值均为 9 个文件准备任务、1 个目录 reader；这测量的是当前实际代码，暂未启用双目录读前。PostgreSQL `baseline_query` 三轮为 10.360 / 1.103 / 2.129 s，缓存和运行抖动明显；正向索引事务 `positive_index_apply` 中位数为 6.459 s，是当前 PG 主要成本之一。相较 LUX-270 的 SQLite 2.018 s / PostgreSQL 9.657 s 参考中位数，本次诊断版本为 2.058 s / 10.472 s，尚未通过阶段性能目标。

### LUX-273 双 reader 流水线 A/B 与回退决定

2026-09-26 在同一 Apple M4 / 16 GiB ARM64、Rust 1.97.1 和 60,000 文件 / 600 目录 fixture 上，对比 LUX-272 顺序 reader 和双 reader、有界 read/prepare/单 writer 流水线各三轮。两组使用相同 fixture SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`；PostgreSQL 使用本机一次性 PostgreSQL 16 容器。

| 后端/实现 | 首扫索引完成：三轮 / 中位数 | SQL / DML 中位数 | 正向提交批次 | target 物化 / 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | WAL 中位数 |
|---|---:|---:|---:|---:|---:|---:|
| SQLite 顺序 reader（LUX-272） | 2.058 / 2.009 / 2.178 s；**2.058 s** | 675 / 209 | 10 | 546 / 961 ms | 259 / 383 ms | — |
| SQLite 流水线 | 1.951 / 2.136 / 1.935 s；**1.951 s** | 832 / 359 | 28 | 598 / 842 ms | 237 / 365 ms | — |
| PostgreSQL 顺序 reader（LUX-272） | 19.059 / 10.472 / 10.368 s；**10.472 s** | 694 / 209 | 10 | 2,523 / 5,145 ms | 281 / 685 ms | 216,506,975 bytes |
| PostgreSQL 流水线 | 10.987 / 17.588 / 14.756 s；**14.756 s** | 871 / 359 | 28 | 2,593 / 3,735 ms | 269 / 620 ms | 228,425,152 bytes |

六轮流水线基准均观察到两个活动 reader、两个并发目录读操作以及读/准备、读/提交重叠，在途峰值 7,454 / 8,192。它把 SQLite 首扫中位数缩短约 5.2%，但 SQL 增约 23%、DML 增约 72%；PostgreSQL 首扫中位数慢约 40.9%，SQL 增约 25%、DML 增约 72%，WAL 增约 5.5%。候选代码已按 LUX-273 条件移除：PostgreSQL 没有稳定收益，单后端加速不足以抵消另一后端回退。SQLite 与 PostgreSQL 数据仍只代表这台 ARM64 开发机和本机测试容器。

### LUX-274 正向索引写入子阶段诊断

2026-09-26 在同一 60,000 文件 / 600 目录 fixture 上各运行一轮，基于 `93a9fa3a` 的顺序 reader 路径，只增加固定名称的子阶段计时。单轮数据用于定位热点，不替代 LUX-272/LUX-273 的三轮中位数；PostgreSQL 此轮总耗时波动尤其明显。

| 后端 | 首扫索引 | `positive_index_apply` | add filesystem claim | add movie materialization | SQL / DML | target 物化 / 无变化重扫 | 前台 p95 | WAL |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite | 2.537 s | 0.978 s | 0.196 s | 0.755 s | 675 / 209 | 0.569 / 0.970 s | 234 ms | — |
| PostgreSQL 16 | 15.808 s | 6.212 s | 0.881 s | 5.260 s | 698 / 209 | 2.673 / 3.815 s | 278 ms | 216,191,375 bytes |

首扫 DML 摘要中，批量 `media_items` 插入 48 次、`media_sources` 插入 38 次、`filesystem_entries` 插入 38 次。PG 锁采样观察到 0 个最大等待者。电影项/来源物化占正向索引阶段的大部分耗时；LUX-274 试验了受参数上限约束的后端批次，SQL 数据合同和事务边界保持不变。单轮 PostgreSQL 数值不能与三轮中位数直接比较。

#### 后端有界批次三轮对比

每轮均使用 60,000 文件 / 600 目录的相同 fixture；LUX-274 顺序 reader 候选分三次使用新建空 PostgreSQL 数据库运行。SQLite 继续使用 2,000 行批次（最多 22,000 个 `media_sources` bind）；PostgreSQL 使用 5,000 行批次（最多 55,000 个 bind，低于 65,535 的参数上限）。SQLite 的 SQL 批次和 DML 数量保持不变。

| 后端/实现 | 首扫索引：三轮 / 中位数 | 正向索引 apply 中位数 | SQL / DML 中位数 | target / 无变化重扫中位数 | 前台 p95 中位数 | WAL 中位数 |
|---|---:|---:|---:|---:|---:|---:|
| SQLite LUX-272 基线 | 2.058 / 2.009 / 2.178 s；**2.058 s** | 0.950 s | 675 / 209 | 0.546 / 0.961 s | 259 ms | — |
| SQLite LUX-274 候选 | 2.539 / 2.111 / 2.146 s；**2.146 s** | 0.969 s | 675 / 209 | 0.570 / 0.986 s | 255 ms | — |
| PostgreSQL 16 LUX-273 顺序基线 | 19.059 / 10.472 / 10.368 s；**10.472 s** | 6.459 s | 694 / 209 | 2.523 / 5.145 s | 281 ms | 216,506,975 bytes |
| PostgreSQL 16 LUX-274 候选 | 9.692 / 9.354 / 8.758 s；**9.354 s** | 6.441 s | 616 / 152 | 2.617 / 3.847 s | 271 ms | 222,461,224 bytes |

PostgreSQL 候选将总 DML 减少约 27%、SQL 减少约 11%，三类主要批量写入分别从 48/38/38 次降为 29/19/19 次；索引完成中位数快约 10.7%，无变化重扫快约 25%。WAL 增加约 2.7%，锁等待采样最大 waiter 数仍为 0；正向索引阶段耗时基本持平，说明数据库行/索引写入仍是剩余成本。SQLite 仍走原 2,000 行批次，DML 不变；其首扫中位数比 LUX-272 参考高约 4.3%，target 和无变化重扫分别高约 4.4% 和 2.6%，前台 p95 改善约 1.5%。这组 ARM64 结果不证明 NAS 性能，也没有关闭 LUX-275 的严格双后端性能门。

### PostgreSQL provider 派生索引 statement trigger 复测

提交 `328d034b` 的迁移 `0141_statement_provider_index_refresh.sql` 将 `media_item_provider_ids` 的 INSERT/UPDATE 触发器改为 statement-level transition table。它避免大批量 `media_items` 写入时为每一行单独执行 provider 索引刷新；SQLite 没有对应迁移。迁移契约、空库启动、旧库升级和 provider 插入/更新语义测试均通过。

2026-09-26 在同一 Apple M4 ARM64、60,000 文件 / 600 目录 fixture、本机 PostgreSQL 16 容器上运行三轮。下面的数值用于定位剩余写入热点；由于尚未在同一环境对旧 row-trigger 版本完成三轮 A/B，不把它们表述为已证实的加速百分比。

| 指标 | 三轮 / 中位数 |
|---|---:|
| 首扫索引完成 | 6.266 / 6.172 / 5.957 s；**6.172 s** |
| `positive_index_apply` | 4.807 / 4.744 / 4.527 s；**4.744 s** |
| `movie_item_insert` | 2.186 / 2.213 / 2.098 s；**2.186 s** |
| `movie_source_insert` | 1.333 / 1.223 / 1.171 s；**1.223 s** |
| filesystem claim | 0.918 / 0.939 / 0.888 s；**0.918 s** |
| target 物化 | 2.412 / 2.380 / 2.502 s；**2.412 s** |
| 无变化重扫 | 2.285 / 2.037 / 2.102 s；**2.102 s** |
| 前台 p95 | 319 / 271 / 268 ms；**271 ms** |
| WAL | 约 219 MB |

同一扫描代码的 SQLite 对照（三轮，关闭 lock monitor）为首扫 2.683 / 2.964 / 2.893 s（中位数 2.893 s）、target 715 ms、无变化重扫 1.032 s、前台 p95 237 ms；provider 迁移未改变 SQLite 结果。以上数据只代表本机 ARM64，不外推 NAS/x86_64，且不关闭 LUX-275 阶段门。

迁移 `0142_filter_available_source_promotions.sql` 继续压缩 source INSERT 的 availability 路径：先筛选仍为 `has_available_source = 0` 的 item，再连接 `filesystem_entries`。同一 Apple M4 / PostgreSQL 16 / 60,000 文件 fixture 的三轮为首扫 6.596 / 6.384 / 6.578 s（中位数 6.578 s）、`movie_source_insert` 1.236 / 1.170 / 1.154 s（中位数 1.170 s）、`positive_index_apply` 5.131 / 4.741 / 4.965 s（中位数 4.965 s）、target 2.378 / 2.412 / 2.373 s（中位数 2.378 s）、无变化重扫 2.214 / 2.292 / 2.295 s（中位数 2.292 s）。与上一组三轮相比没有形成稳定的总耗时加速，因此保留它作为无语义变化的冗余探测削减，不把它计入 LUX-275 性能门收益。

### PostgreSQL media_search/provider 刷新合并与未使用索引清理（0143）

迁移 `0143_merge_provider_refresh_into_media_search.sql` 将 provider lookup 刷新合并到已有的 `media_items` statement-level search trigger，并删除实际 `ILIKE '%term%'` 查询不使用的 `media_search(title)` 与 `media_search(sort_title)` B-tree。标题未变化时只在搜索行缺失的情况下补建；provider 字段未变化时不删除/重建 provider 行。SQLite 路径不变。

2026-09-26 在同一 Apple M4 ARM64、PostgreSQL 16 本机容器、60,000 文件 / 600 目录 fixture 上关闭锁采样，分别对干净 0142 worktree 和 0143 工作树各运行三轮。fixture SHA-256 为 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`。

| 版本 | 首扫索引完成：三轮 / 中位数 | `positive_index_apply` 中位数 | target 物化 | 无变化重扫 | 前台 p95 | SQL / DML | WAL 中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 0142 基线 | 15.242 / 15.266 / 15.015 s；**15.242 s** | 4.441 s | 2.394 s | 12.103 s | 267 ms | 421 / 122 | 219,210,636 bytes |
| 0143 工作树 | 14.771 / 13.171 / 14.642 s；**14.642 s** | 4.250 s | 2.378 s | 12.116 s | 272 ms | 419 / 122 | 209,838,782 bytes |

相对同机 0142 A/B，首扫中位数下降约 3.9%，`positive_index_apply` 下降约 4.3%，WAL 下降约 4.3%；无变化重扫增加约 0.1%，前台 p95 增加约 1.9%，均在 5% 回退门槛内。这个改动确认减少了 PostgreSQL 派生写入成本，但绝对首扫仍高于 LUX-270 的 9.657 秒参考，因此不关闭 LUX-275，也不能外推 NAS/x86_64。

### Manifest-lite 目录 frontier A/B（探索性）

2026-09-26 在同一 Apple M4 / 16 GiB ARM64、60,000 个文件 / 600 个目录 fixture 上，对比新扫描默认的 `workflow_version=2`、`discovery_format_version=3`、`discovery_mode=LITE` 与旧持久 frontier 路径。Lite 将目录 frontier 保存在进程内，子目录不写入 `scan_manifest_directories`；format 3 的文件存在性仍使用 `last_seen_generation` 和紧凑 seen-path ledger。SQLite 运行三轮，PostgreSQL 16 使用本机一次性容器运行一轮；旧持久 frontier 也各运行一轮，因此这组数据用于定位收益，不替代 LUX-275 的三轮同构 A/B。

| 后端/路径 | 首扫索引完成 | 正向提交批次；外层批次 | 无变化重扫 | 前台 p95 | WAL / 锁等待 |
|---|---:|---:|---:|---:|---:|
| SQLite Lite | 2.606 / 2.673 / 2.862 s；**2.673 s** | 8；11 | 约 1.03 s | 约 239–247 ms | — |
| SQLite 旧持久 frontier | 4.310 s | — | — | — | — |
| PostgreSQL 16 Lite | 6.051 s | — | 3.389 s | 274 ms | 225,100,660 bytes；0 |
| PostgreSQL 16 旧持久 frontier | 14.989 s | — | — | — | — |

Lite 的收益主要来自移除逐目录 frontier 的数据库写入和恢复查询；它不表示 Manifest 的全部语义可以删除。SQLite 当前中位数仍高于 LUX-270 的 2.018 秒参考，PostgreSQL 只有单轮结果，且所有数据只代表本机 ARM64 和临时数据库，不能关闭 LUX-275 或外推 NAS/x86_64。

实现与语义边界见 `docs/decisions/045-manifest-lite-discovery.md`。

### Jellyfin 风格目录批处理（已否决的历史对照）

2026-09-27 在同一台 Apple M4 / 16 GiB / ARM64 机器上，以相同的 60,000 文件、600 目录 fixture（SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`）交替运行三轮 Lite 与 Jellyfin 风格路径。SQLite 使用 `synchronous=FULL`，关闭会每 100 ms 写入一次的 SQLite 锁采样器；PostgreSQL 使用本机 Docker PostgreSQL 16.15 和每轮全新数据库，保留锁等待采样。两种路径都生成 120,000 个 targets，并在扫描期间采样 50 个前台请求。

Jellyfin 当前代码先完整收集一个目录的 child snapshot，再对同目录新增项成组 `CreateItems`，然后继续递归验证目录；`DirectoryService` 另有扫描期间的目录项、文件元数据和路径缓存。[Folder.cs](https://github.com/jellyfin/jellyfin/blob/390296c9c8160bb6ad6f01b41226398776d21a83/MediaBrowser.Controller/Entities/Folder.cs)、[LibraryManager.cs](https://github.com/jellyfin/jellyfin/blob/390296c9c8160bb6ad6f01b41226398776d21a83/Emby.Server.Implementations/Library/LibraryManager.cs)、[DirectoryService.cs](https://github.com/jellyfin/jellyfin/blob/390296c9c8160bb6ad6f01b41226398776d21a83/MediaBrowser.Controller/Providers/DirectoryService.cs)。Lux 曾实现一个仅供对照的原型：按父目录分别解析和提交，每目录约 100 个文件，因此同目录文件在一个有界批次内完成准备并单独提交；更大的目录仍按 Lux 的 chunk 上限流式处理。原型复用 Lux 的 Manifest、二次 stat、CAS、root 删除门槛和 target barrier，没有移植 Jellyfin 的 metadata/provider 对象模型，也没有引入其 scan-scoped cache。基于下方数据，逐目录提交显著增加数据库往返与事务固定开销；该原型和 feature 已从当前代码中移除，以下结果仅作为已否决方案的历史对照。

| 后端 / 方案 | 首扫索引完成（三轮；中位数） | 120k target 物化中位数 | 无变化重扫（三轮；中位数） | 正向提交批次 | SQL / DML | batch p95 | 前台请求 p95 | WAL 中位数 / 最大锁 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite Lite grouped | 2.880 / 2.335 / 2.300 s；**2.335 s** | 0.753 s | 1.027 / 1.044 / 0.968 s；**1.027 s** | 8 | 385 / 128 | 328 ms | 242 ms | — |
| SQLite Jellyfin folder batch | 2.744 / 2.688 / 2.708 s；**2.708 s** | 0.752 s | 9.737 / 9.842 / 9.877 s；**9.842 s** | 600 | 8,583 / 4,819 | 377 ms | 246 ms | — |
| PostgreSQL 16 Lite grouped | 6.026 / 6.124 / 5.881 s；**6.026 s** | 2.907 s | 3.380 / 3.091 / 3.138 s；**3.138 s** | 8 | 353 / 104 | 796 ms | 281 ms | 222,668,454 bytes / 0 |
| PostgreSQL 16 Jellyfin folder batch | 66.313 / 62.534 / 63.054 s；**63.054 s** | 2.308 s | 11.119 / 4.614 / 6.148 s；**6.148 s** | 600 | 9,215 / 4,819 | 15,831 ms | 270 ms | 227,071,287 bytes / 0 |

在这组均匀分布 fixture 上，目录模式让 SQLite 首扫慢约 16%，无变化重扫约 9.6 倍；PostgreSQL 首扫约 10.5 倍、重扫约 2 倍。PostgreSQL 的 batch p95 从 796 ms 增至 15.8 s。阶段计时也指向数据库路径：PostgreSQL `baseline_query` 累计中位数从 96 ms 增至 22.68 s，`positive_index_apply` 从 4.49 s 增至 31.25 s；目录 open/readdir/stat 累计合计约从 154 ms 增至 754 ms。阶段值是累计工作时间，不能相加当作墙钟。SQL/DML 增长与 8 → 600 个提交一致，而 WAL 仅增加约 2%，锁 waiter 仍为 0；主要代价是逐目录数据库往返和事务固定开销，不是锁争用。target 物化时间独立测量，不计入首扫索引完成时间。

历史方案供定位，不与上面的同构三轮 A/B 混算：

| 历史方案 | SQLite 首扫 | PostgreSQL 首扫 | 说明 |
|---|---:|---:|---|
| LUX-045 直接扫描（提交 `4802c939`） | 2.105 s | — | 旧版记录；不含本轮 Manifest / 120k target 完成口径 |
| 旧持久 frontier（2026-09-26 单轮） | 4.310 s | 14.989 s | 没有同轮重扫样本 |
| 最初 Manifest 实现 | 202.468 s | 142.229 s | LUX-270 初版，后来已大幅收敛 SQL/DML |

本轮又尝试运行当前工作树的 LUX-045 全流程基准；进程满核超过 5 分钟仍未结束，遂停止，未形成有效计时。因此历史 LUX-045 的 2.105 秒只能作为旧版本参考，不能宣称当前直接扫描快于 Manifest Lite。

结论：不采用逐目录提交方案，相关实现已移除。后续优化继续以 Lite 的有界跨目录读取和批量提交为基线；目录快照或 scan-scoped 文件系统缓存只有在独立测量证明收益后再考虑。该历史实验不关闭 LUX-275 阶段门，也不代表 NAS/x86_64 性能。

### NEW target 物化快速路径

2026-09-27 对 Lite grouped 的 target 物化增加 NEW 阶段快速路径：该阶段的输入已经限定为 `last_seen_change_kind = 'NEW'`，因此 ITEM target 直接写入 `NEW`，省去逐 item 查询同一 generation 是否存在 NEW source 的相关 `EXISTS`。CHANGED 阶段仍保留原判定，确保一个 item 同时含有 NEW 与 CHANGED source 时，ITEM target 仍优先标记 NEW。上方 Jellyfin A/B 的 Lite target 时间作为改动前同机基线。

同一 Apple M4 / 16 GiB / ARM64、本机 PostgreSQL 16.15、相同 60k/600 fixture 和 SQLite `synchronous=FULL` 配置下，候选代码分别运行三轮；每轮 PostgreSQL 使用新数据库，SQLite 关闭每 100 ms 写事务的锁采样器。结果只将 target 阶段与改动前基线比较：

| 后端 | 指标 | 改动前三轮中位数 | 快速路径三轮原始值 | 快速路径中位数 | 差异 |
|---|---|---:|---:|---:|---:|
| SQLite | 120k target 物化 | 0.753 s | 0.683 / 0.704 / 0.709 s | 0.704 s | 快约 6.5% |
| PostgreSQL 16 | 120k target 物化 | 2.907 s | 2.676 / 2.674 / 2.602 s | 2.674 s | 快约 8.0% |

快速路径的 SQLite target SQL/DML 为 94/34 条，PostgreSQL 为 103/34 条；PostgreSQL WAL 三轮中位数为 221,962,439 bytes，最大锁 waiter 为 0。候选运行的扫描索引中位数为 SQLite 2.898 s、PostgreSQL 6.216 s，无变化重扫为 0.993 s、3.179 s。索引计时在 target 代码运行之前结束，这段改动不会进入索引或重扫路径；这些跨时段差值不能归因于快速路径，也不能据此宣称首扫总时间改善。LUX-275 阶段门仍开放。

### 合并 target 写入与 16k 有界页

随后将每个游标页的 SOURCE 和 ITEM target 合并为同一条 `INSERT ... SELECT ... UNION ALL`，复用一次物化的 source page；CHANGED 页仍按 generation 检查 NEW 优先级。页大小从 8,000 调到 16,000 个 source，单次语句写入两类 target。下面比较同一候选路径的 8k 与 16k；硬件为 Apple M4 / 16 GiB / ARM64，fixture 为 60,000 files / 600 directories，SQLite 使用 `synchronous=FULL` 并关闭锁采样，PostgreSQL 16.15 每轮使用新临时库并采集 WAL 与锁等待。

| 后端 / 页大小 | 索引完成中位数 | 120k targets 中位数 | 无变化重扫中位数 | 前台 50 请求 p95 中位数 | target SQL / DML | target INSERT 次数 | batch p95 中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|
| SQLite / 8k | 2.876 s | 684 ms | 961 ms | 237 ms | 86 / 26 | 8 | 未记录 |
| SQLite / 16k | 2.914 s | 629 ms | 962 ms | 234 ms | 50 / 14 | 4 | 401 ms |
| PostgreSQL 16 / 8k | 6.666 s | 2.660 s | 3.346 s | 283 ms | 95 / 26 | 8 | 933 ms |
| PostgreSQL 16 / 16k | 6.617 s | 2.466 s | 2.997 s | 263 ms | 55 / 14 | 4 | 937 ms |

16k 页使 target 阶段相对同一合并语句的 8k 页在 SQLite 快约 8.0%、PostgreSQL 快约 7.3%，并将 target INSERT 次数减半；索引、无变化重扫、前台 p95 与 batch p95 均未出现超过 5% 的回退。16k 的 PostgreSQL WAL 中位数为 207,871,938 bytes，三轮最大锁 waiter 为 0。SQLite 索引中位数 2.914 s 仍高于 LUX-270 的 2.018 s 参考，target 调整也不会改变索引计时范围；因此 LUX-275 严格门继续开放。此实验只代表本机 ARM64 与临时 PostgreSQL，不外推 NAS/x86_64。

### Lite 根目录状态更新去重

Lite 不把子目录 frontier 写入 `scan_manifest_directories`，但此前每个正向提交批次只要完成了任意目录，就会再次尝试更新该表中的根目录行；其 `state <> 'COMPLETE'` 条件让后续调用成为无效果 UPDATE。现在只在 Lite frontier 清空的收尾事务中更新根目录状态。对同一 60k/600 fixture 的 SQLite release 单轮测量，Lite 根目录状态 UPDATE 从 10 条降为 1 条，扫描 DML 从 128 条降为 119 条；首扫为 2.894 s、无变化重扫 969 ms、前台 p95 231 ms。首扫单轮与此前 2.914 s 三轮中位数基本相同，故仅记录冗余 SQL 的减少，不把它算作稳定加速。PostgreSQL 单轮为首扫 6.709 s、target 2.496 s、无变化重扫 3.212 s、前台 p95 259 ms、batch p95 932 ms，WAL 208,039,460 bytes、锁等待为 0；这组单轮只用于确认该路径可运行，不作为性能差异结论。LUX-275 仍开放。

当前基准入口只运行 Lite 路径。复跑时使用同一个 60k fixture；SQLite 设 `LUX_PERF_BACKEND=sqlite LUX_PERF_DISABLE_LOCK_MONITOR=1`，PostgreSQL 设 `LUX_PERF_BACKEND=postgres POSTGRES_TEST_DATABASE=<disposable-empty-db>`。release test 命令为：

```bash
CARGO_TARGET_DIR=/Volumes/Toshiba/mywork/Lux/target \
cargo test --release --locked \
  --test performance lux_270_manifest_job_scan_benchmark -- \
  --ignored --nocapture --test-threads=1
```

### LUX-275 两目录首批预读 A/B（已否决实验）

2026-09-27 对最近已提交的顺序目录 reader（`7952e4e5`）与候选实现做交错 release A/B。两版使用同一 SHA-256 为 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914` 的 60,000 文件 / 600 目录 fixture，每后端各三轮；每轮交替先跑的版本。硬件为 Apple M4 / 16 GiB（`uname -m=arm64`），PostgreSQL 为本机 PostgreSQL 16.15 容器、每次首扫使用独立空库；SQLite 使用 `synchronous=FULL` 并关闭 100 ms 锁采样器，PostgreSQL 保留锁等待采样。

候选只并行打开两个目录并预读各自第一页，处理仍按目录原顺序交给同一个 writer。reader 总数上限为 2，每路首批最多 4,000 个文件，数据库正向提交仍按 8,000 文件分批；没有引入读写流水线或并行事务。

| 后端 / reader | 首扫索引：三轮 / 中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 | 正向提交批次 | WAL 中位数 / 最大 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite 顺序 | 2.763 / 2.177 / 2.246 s；**2.246 s** | 620 ms | 976 ms | 238 / 375 ms | 308 ms | 376 / 119 | 8 | — |
| SQLite 两目录首批预读 | 2.217 / 2.244 / 2.201 s；**2.217 s** | 628 ms | 933 ms | 236 / 383 ms | 312 ms | 376 / 119 | 8 | — |
| PostgreSQL 16 顺序 | 5.898 / 6.113 / 6.008 s；**6.008 s** | 2.513 s | 3.037 s | 264 / 309 ms | 796 ms | 344 / 95 | 8 | 210,321,664 bytes / 0 |
| PostgreSQL 16 两目录首批预读 | 5.778 / 5.791 / 5.805 s；**5.791 s** | 2.483 s | 2.955 s | 264 / 300 ms | 776 ms | 344 / 95 | 8 | 210,333,350 bytes / 0 |

预读实验的首扫中位数相对同机顺序版快约 1.3%（SQLite）和 3.6%（PostgreSQL）；target、无变化重扫、前台 p95 和 batch p95 中位数均未超过 5% 回退，DML、正向提交批次和 WAL 基本不变。SQLite 有一轮顺序版比预读版慢约 20%，另两轮差距约为 3% 内；因此 1.3% 的中位数变化没有越过本机运行波动。综合收益和增加的 reader 调度复杂度，不保留预读候选；代码继续使用顺序 reader。顺序版首扫中位数 2.246 秒仍比 LUX-270 的 2.018 秒参考慢约 11.3%，阶段 22 / LUX-275 性能门继续开放。这些本机 ARM64 结果不外推 NAS/x86_64。

### LUX-275 32k target page A/B

2026-09-27 对 16k 与 32k postprocessing target page 做交错 release A/B，各后端各三轮，使用与上节相同的 60k/600 fixture 和 Apple M4 / 16 GiB ARM64 环境。PostgreSQL 为每轮新建的本机 16.15 空库；SQLite 为 `synchronous=FULL` 且关闭锁采样器。32k 页仍有硬上限；查询用 seek cursor 和 LIMIT 取路径，SQL 每页只绑定固定数量的游标、generation 与 page limit，没有逐行 bind 参数。每页的 SOURCE 与 ITEM target 仍在单一 SQL、单一事务内一起提交，ready barrier 仍在所有根完成后推进。

| 后端 / page size | 首扫索引：三轮 / 中位数 | 120k target 物化：三轮 / 中位数 | 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | batch p95 中位数 | target SQL / DML | target INSERT | WAL 中位数 / 最大 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite / 16k | 2.788 / 2.252 / 2.276 s；**2.276 s** | 617 / 630 / 635 ms；**630 ms** | 968 ms | 237 / 375 ms | 324 ms | 50 / 14 | 4 | — |
| SQLite / 32k | 2.173 / 2.185 / 2.155 s；**2.173 s** | 560 / 568 / 556 ms；**560 ms** | 976 ms | 234 / 381 ms | 299 ms | 32 / 8 | 2 | — |
| PostgreSQL 16 / 16k | 5.919 / 6.062 / 6.009 s；**6.009 s** | 2,400 / 2,566 / 2,549 ms；**2,549 ms** | 3,092 ms | 279 / 311 ms | 789 ms | 55 / 14 | 4 | 207,410,538 bytes / 0 |
| PostgreSQL 16 / 32k | 5.862 / 5.904 / 5.828 s；**5.862 s** | 2,629 / 2,356 / 2,523 ms；**2,523 ms** | 3,000 ms | 286 / 312 ms | 801 ms | 35 / 8 | 2 | 210,311,667 bytes / 0 |

32k 页把每后端的 target INSERT 从 4 条减到 2 条，target DML 从 14 条降至 8 条。target 阶段中位数 SQLite 快约 11.1%，PostgreSQL 快约 1.0%；PostgreSQL WAL 增约 1.4%，最大 waiter 仍为 0。无变化重扫、前台 p95、目录列表 p95 和 batch p95 中位数均未超过 5% 回退。首扫在 target 阶段之前已经计时，表中首扫差异是运行波动，不能归因于 page size。SQLite 仍未满足 LUX-270 的 2.018 秒索引完成参考，LUX-275 阶段门保持开放；这些本机数据不外推 NAS/x86_64。

### LUX-275 SQLite 搜索触发器空 alias 查询 A/B

2026-09-27 对提交 `71982fef` 的旧 INSERT trigger 与候选 migration `0146_skip_empty_alias_lookup_on_media_item_insert.sql` 做三轮交错 release 基准。旧 trigger 每插入一个媒体条目都会按 `item_id` 查询 `item_aliases` 并执行 `group_concat`；新媒体条目受外键保护，不可能在 INSERT trigger 前已有 alias，后续 alias 的插入/更新/删除仍由原有 alias trigger 刷新全文索引。新库启动时还会由 SQLite 兼容修复重建 `media_items`；因此 `src/storage/migration.rs` 中重建该 trigger 的定义也同步改为 `''`。

两版共用 SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914` 的 60,000 文件 / 600 目录 fixture，Apple M4 / 16 GiB / ARM64；SQLite 使用 `synchronous=FULL` 并关闭 100 ms 锁采样，PostgreSQL 使用本机 Docker PostgreSQL 16.15，每轮新建空数据库并保留锁等待采样。每次运行包含 120k target 物化、无变化重扫和扫描期间的 50 个前台请求。性能二进制直接执行 `lux_270_manifest_job_scan_benchmark --ignored --nocapture --test-threads=1`；原始 release test 总墙钟包括以上各阶段及基准初始化。

| 后端 / trigger | 首扫索引：三轮 / 中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 | 基准总墙钟中位数 | WAL 中位数 / 最大 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite / 旧版 | 2.750 / 2.236 / 2.216 s；**2.236 s** | 581 ms | 978 ms | 240 ms | 322 ms | 376 / 119 | 4.25 s | — |
| SQLite / 空 alias 快速路径 | 2.085 / 2.157 / 2.069 s；**2.085 s** | 558 ms | 968 ms | 240 ms | 288 ms | 376 / 119 | 4.04 s | — |
| PostgreSQL 16 / 旧版 | 5.855 / 5.876 / 5.831 s；**5.855 s** | 2,327 ms | 3,061 ms | 275 ms | 780 ms | 344 / 95 | 13.49 s | 210,384,596 bytes / 0 |
| PostgreSQL 16 / SQLite-only migration | 5.887 / 5.822 / 5.984 s；**5.887 s** | 2,387 ms | 3,097 ms | 277 ms | 791 ms | 344 / 95 | 13.51 s | 222,086,857 bytes / 0 |

SQLite 首扫索引中位数快约 6.7%，基准总墙钟快约 4.9%；无变化重扫和前台 p95 基本持平，batch p95 下降约 10.6%，SQL/DML 数量不变。`positive_index_apply` 累计中位数为 1.076 → 1.062 秒；该值是批次累计工作时间，不是墙钟时间。首扫 2.085 秒仍比 LUX-270 的 2.018 秒参考慢约 3.3%，所以没有关闭 LUX-275。

PostgreSQL 代码路径未被这项 SQLite 优化修改；首扫中位数变化约 +0.5%，重扫、前台 p95 和 batch p95 均在 5% 以内，最大锁 waiter 为 0。WAL 中位数观察到 210.4 → 222.1 MB（约 +5.6%）；本轮 PG SQL/DML 计数相同，且每轮使用随机生成的条目 ID，因此该 WAL 差异的成因未由这次 A/B 确认，不归因于 SQLite migration。结果仅代表本机 ARM64 和临时数据库，不外推 NAS/x86_64。

复跑沿用本节前的 release 命令及同一 fixture；SQLite 设置 `LUX_PERF_BACKEND=sqlite LUX_PERF_SQLITE_SYNCHRONOUS=FULL LUX_PERF_DISABLE_LOCK_MONITOR=1`，PostgreSQL 设置 `LUX_PERF_BACKEND=postgres POSTGRES_TEST_DATABASE=<disposable-empty-db>`。候选版改动文件为 `migrations/0146_skip_empty_alias_lookup_on_media_item_insert.sql` 和 `src/storage/migration.rs`；alias 检索回归由 `tests/search.rs::fts_search_matches_chinese_titles_and_aliases_with_acl` 覆盖。

### SQLite provider-ID INSERT trigger 短路 A/B（未保留）

2026-09-27 在提交 `9cf43819` 上评估给 SQLite `media_item_provider_ids_ai` 增加 `WHEN NEW.provider_ids_json IS NOT NULL`，避免无 provider ID 的新条目执行一次 `json_each('{}')`。候选包含升级 migration 和启动时兼容重建修正；功能测试确认 providerless item 不生成派生索引行，实际 TMDB ID 仍进入索引。候选最后未保留，因为全链路没有改善。

基准继续使用同一 Apple M4 / 16 GiB / ARM64 和 60k/600 fixture（SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`），SQLite `synchronous=FULL`、锁采样关闭，每版三轮：

| SQLite trigger | 首扫索引：三轮 / 中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 中位数 | batch p95 中位数 | SQL / DML | 基准总墙钟中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 原 trigger | 2.156 / 2.254 / 2.288 s；**2.254 s** | 580 ms | 1,010 ms | 247 ms | 320 ms | 376 / 119 | 4.29 s |
| providerless 短路候选 | 2.259 / 2.314 / 2.280 s；**2.280 s** | 583 ms | 983 ms | 253 ms | 326 ms | 376 / 119 | 4.35 s |

候选的 `movie_item_insert` 累计时间中位数从 459.0 降到 449.5 ms，但 `positive_index_apply` 基本持平（1,085.8 → 1,083.3 ms），首扫反而慢约 1.2%，基准总墙钟慢约 1.4%。因此 migration、兼容重建改动及其临时测试均已撤回；不将子阶段变化当作全链路收益。PostgreSQL 未重测，因为它不使用此 SQLite trigger 路径。

### SQLite/PostgreSQL 层级索引前缀去重 A/B（未保留）

2026-09-27 在 `67e1a0db` 基线上评估删除 `media_items(parent_id, removed_at)` 和 `(series_id, removed_at)` 两个窄索引；它们分别被 `(parent_id, removed_at, has_available_source)` 和 `(series_id, removed_at, has_available_source)` 的前缀覆盖。候选同步更新了 SQLite 兼容表重建逻辑，并由空库迁移删除两条旧索引。SQLite schema/EXPLAIN 回归和 PostgreSQL 空库启动迁移测试通过，确认层级查询仍命中保留的复合索引。

基线与候选在 Apple M4 / 16 GiB / ARM64、同一 SHA-256 为 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914` 的 60,000 文件 / 600 目录 fixture 上交错各跑三轮 release 基准。SQLite 为 `synchronous=FULL`、关闭锁采样；PostgreSQL 为本机 Docker 16.15，每轮使用新空库并采样锁等待。每轮还测 120k targets、无变化重扫和 50 并发前台请求。

| 后端/索引 | 首扫索引完成：三轮 / 中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 中位数 | 目录列表 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 | WAL 中位数 / 最大 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| SQLite / 基线 | 2.334 / 2.259 / 2.277 s；**2.277 s** | 578 ms | 1,007 ms | 230 ms | 366 ms | 324 ms | 376 / 119 | — |
| SQLite / 删除两条前缀索引 | 2.669 / 2.330 / 2.165 s；**2.330 s** | 564 ms | 984 ms | 236 ms | 375 ms | 319 ms | 376 / 119 | — |
| PostgreSQL 16 / 基线 | 5.894 / 5.968 / 6.041 s；**5.968 s** | 2,503 ms | 3,094 ms | 266 ms | 339 ms | 800 ms | 344 / 95 | 213,008,651 bytes / 0 |
| PostgreSQL 16 / 删除两条前缀索引 | 5.962 / 5.824 / 6.133 s；**5.962 s** | 2,406 ms | 3,090 ms | 264 ms | 356 ms | 812 ms | 344 / 95 | 198,472,015 bytes / 0 |

两后端首扫索引完成都没有稳定改善：SQLite 候选中位数慢约 2.3%，PostgreSQL 仅快约 0.1%。target 阶段略快，但不在首扫索引计时内；PG WAL 中位数低约 6.8%，现有三轮无法排除随机数据与写入波动，不能归因于索引删除。为遵守 LUX-275 的端到端收益门，候选 migration、兼容重建改动与测试均撤回；这一候选不保留为性能优化。数据只代表本机 ARM64，不外推 NAS/x86_64。

### PostgreSQL providerless INSERT trigger 快速路径 A/B

2026-09-27 在 `209b8754` 基线上评估 PostgreSQL migration `0146_skip_empty_provider_index_expansion.sql`。已有 statement-level `media_items` INSERT trigger 会把 transition table 每一行传给 `json_each_text`；扫描新媒体条目通常 `provider_ids_json IS NULL` 或 `{}`，因此候选先物化并过滤出非空 provider JSON，再调用 JSON table function。搜索索引仍为所有新条目写入，provider 派生索引只为非空 JSON 写入。SQLite 代码和 schema 未变。

基线和候选在 Apple M4 / 16 GiB / ARM64、60,000 文件 / 600 目录 fixture（SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914`）上交错运行三轮 release 基准。PostgreSQL 为 Docker 16.15，每轮新空库并采样锁等待；SQLite 使用 `synchronous=FULL` 并关闭锁采样。每轮包括 120k target 物化、无变化重扫和 50 个前台请求。

| PostgreSQL 16 | 首扫索引完成：三轮 / 中位数 | `movie_item_insert` 累计中位数 | `positive_index_apply` 累计中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 中位数 | 目录列表 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 | WAL 中位数 / 最大 waiter |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 基线 | 6.143 / 6.184 / 5.993 s；**6.143 s** | 1,844 ms | 4,530 ms | 2,451 ms | 3,195 ms | 277 ms | 368 ms | 807 ms | 344 / 95 | 220,946,496 bytes / 0 |
| providerless JSON 快速路径 | 6.043 / 5.976 / 5.860 s；**5.976 s** | 1,793 ms | 4,433 ms | 2,453 ms | 3,165 ms | 268 ms | 374 ms | 800 ms | 344 / 95 | 221,833,274 bytes / 0 |

三组配对首扫都变快，候选中位数快约 2.7%；`movie_item_insert` 和 `positive_index_apply` 累计中位数分别下降约 2.8% 和 2.1%。target、无变化重扫、前台 p95 与 batch p95 中位数均未回退超过 5%，最大锁 waiter 为 0。WAL 仅高约 0.4%，不视为确定性变化。候选保留为 PostgreSQL 写入优化；它不改变 SQLite 指标，LUX-275 的 SQLite 首扫门仍开放。结果只代表本机 ARM64 和临时 PostgreSQL 容器，不外推 NAS/x86_64。

### SQLite sort-title FTS 重复 token 削减

2026-09-27 对 SQLite migration `0147_skip_redundant_sort_title_fts_tokens.sql` 做同 fixture 交错三轮 A/B。扫描器新建电影条目的 `sort_title` 通常只是 `title` 的 ASCII 小写形式；FTS5 对 ASCII 大小写不敏感，因此 insert/update trigger 在两者仅有 ASCII 大小写差异时将 FTS 的 `sort_title` 列置空，避免为每个 token 再写一份重复倒排项。判定使用 SQLite `NOCASE`，只跳过可证明安全的 ASCII 大小写差异；其他语言字符或真正不同的排序标题仍完整写入。`title`、`original_title`、aliases 与媒体库排序字段均未改变，已有 FTS 行不重建。

基线为 `5a3e1e5b`，在 Apple M4 / 16 GiB / ARM64 上，针对同一 SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914` 的 60,000 文件 / 600 目录 fixture 交错运行三轮。两版 release 测试二进制固定后复用同一 fixture；SQLite 使用 `synchronous=FULL`、关闭锁采样。每轮包括索引首扫、120k target 物化、无变化重扫和扫描期间 50 个前台请求。

| SQLite FTS trigger | 首扫索引：三轮 / 中位数 | `movie_item_insert` 累计中位数 | `positive_index_apply` 累计中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 基线 | 2,307 / 2,295 / 2,289 ms；**2,295 ms** | 454.0 ms | 1,074.5 ms | 573 ms | 981 ms | 244 / 375 ms | 312 ms | 376 / 119 |
| 跳过重复 sort-title tokens | 2,163 / 2,247 / 2,285 ms；**2,247 ms** | 449.7 ms | 1,065.0 ms | 563 ms | 996 ms | 240 / 381 ms | 310 ms | 376 / 119 |

三组配对首扫均未回退，中位数快约 2.1%；`movie_item_insert` 累计中位数快约 0.9%。target、前台 p95 和 batch p95 均改善，无变化重扫慢约 1.5%，目录列表 p95 慢约 1.6%，均在 5% 回退门内。SQL/DML、批次数和准备并发不变。测试覆盖扫描生成的大小写等价 sort title 仍能通过 title 搜索，以及 update 后真正不同的 sort title 仍能被 FTS 搜索。优化仅影响 SQLite，PostgreSQL 路径未改；小幅收益只代表本机 ARM64，不外推 NAS/x86_64，LUX-275 双后端阶段门继续开放。

测量调用预先构建并固定基线/候选 release `performance` 测试二进制，之后按 `candidate, baseline` 顺序对同一 fixture 交错运行：

```bash
LUX_PERF_MEDIA_ROOT="$FIXTURE" \
LUX_PERF_FILE_COUNT=60000 \
LUX_PERF_BACKEND=sqlite \
LUX_PERF_SQLITE_SYNCHRONOUS=FULL \
LUX_PERF_DISABLE_LOCK_MONITOR=1 \
"$PERFORMANCE_TEST_BINARY" \
  --ignored --nocapture --test-threads=1 lux_270_manifest_job_scan_benchmark
```

### SQLite FTS5 移除未使用的 docsize 表 A/B

2026-09-27 评估 SQLite migration `0148_fts_columnsize_zero.sql`。SQLite FTS5 默认维护 `media_search_docsize`，保存每行每列的 token 数；Lux 搜索只使用 `MATCH`，没有 `bm25()`、`rank`、`snippet()` 或 `highlight()` 调用。候选保持默认 `detail=full` 和现有 tokenizer，只设置 [`columnsize=0`](https://sqlite.org/fts5.html#the_columnsize_option)，并重建索引以移除该 shadow table；迁移时从 `media_items` 和 `item_aliases` 重建现存索引，并继续跳过与标题仅 ASCII 大小写等价的 sort title。既有表字段、标点短语搜索、多个 token 的 AND 语义不变。没有选择 [`detail=column`](https://sqlite.org/fts5.html#the_detail_option)：它不支持短语查询，而 Lux 把一个空格片段整体加引号，标点会被 tokenizer 拆成多个词。

基线为 `ef403125`（含 0147），候选为其上的 0148 migration。在 Apple M4 / 16 GiB / ARM64、同 SHA-256 `23de3a20c11c6a6e7cd44b76af7d1a84e85b9747e2ed2661668dbdf94dad9914` 的 60,000 文件 / 600 目录 fixture 上，对已固定的两版 release 测试二进制交错运行九轮；前六组候选先跑，后三组基线先跑。SQLite 使用 `synchronous=FULL`、关闭锁采样；每轮测索引首扫、120k target、无变化重扫与 50 个前台请求。

| SQLite FTS5 | 首扫索引：九轮 / 中位数 | `movie_item_insert` 累计中位数 | `positive_index_apply` 累计中位数 | 120k target 中位数 | 无变化重扫中位数 | 前台 p95 / 目录列表 p95 中位数 | batch p95 中位数 | SQL / DML 中位数 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 基线 | 2,271 / 2,279 / 2,242 / 2,199 / 2,200 / 2,160 / 2,462 / 2,197 / 2,176 ms；**2,200 ms** | 441.6 ms | 1,046.0 ms | 551 ms | 956 ms | 234 / 379 ms | 304 ms | 376 / 119 |
| `columnsize=0` | 2,710 / 2,137 / 2,196 / 2,232 / 2,067 / 2,099 / 2,015 / 2,156 / 2,031 ms；**2,137 ms** | 428.0 ms | 1,044.1 ms | 552 ms | 972 ms | 238 / 374 ms | 295 ms | 376 / 119 |

九轮首扫中位数快约 2.9%；`movie_item_insert` 累计中位数快约 3.1%。无变化重扫慢约 1.7%、前台 p95 慢约 1.7%，仍低于 5% 回退门；target 基本持平，目录列表 p95 和 batch p95 改善。反向运行的三组配对首扫都更快；九轮中候选有一轮 2,710 ms、基线有一轮 2,462 ms 明显偏慢，故保留原始分布并以中位数报告。SQLite DML 仍为 119，批次数不变。migration 会一次性重建现有 FTS 索引；升级回归确认 title、独立 sort title、original title、alias 均仍可搜索，且重建后没有 `media_search_docsize` 表。PostgreSQL 不使用此 migration；SQLite 中位数 2,137 ms 仍高于 LUX-270 的 2,018 ms 参考，LUX-275 阶段门继续开放。本机 ARM64 结果不外推 NAS/x86_64。
