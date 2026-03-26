# waapi-rs 发布待办清单

## P0 — 发布阻塞项

- [x] **解决 `wamp_async` git 依赖** — 已完成重构，改用 `tokio-tungstenite` 自行实现 WAMP Basic Profile 子集，彻底去除所有 git 依赖，所有依赖均为 crates.io 版本。

- [x] **添加 `license` 字段** — 已添加 `license = "MIT"`

- [x] **添加 LICENSE 文件** — 已创建 MIT LICENSE

- [x] **添加 `description` 字段** — 已添加

## P1 — 功能与改进

- [x] **subscribe 支持传 options 参数** — 已支持，`subscribe(topic, options, callback)` 的 `options: Option<Value>` 可用于过滤、返回字段等

## P1 — 强烈建议

- [x] **补充 `Cargo.toml` 中的 `authors` 格式** — 已修正为标准格式 `"xmimu <1101588023@qq.com>"`

- [x] **README 安装说明更新** — 已更新为 `waapi-rs = "0.2"`（crates.io 版本号依赖）

- [x] **确认公开 API 的文档注释完整** — `cargo doc --no-deps` 无警告，文档生成正常

- [x] **运行 `cargo clippy` 修复所有 lint 警告** — 修复 `uninlined_format_args` 及 5 处 `result_large_err`（`WebSocket` variant 改为 `Box<tungstenite::Error>`），0 警告

- [x] **运行 `cargo test` 确保测试通过** — 全部通过（集成测试 + doctest）

- [ ] **运行 `cargo publish --dry-run` 模拟发布，确认无报错**

## P2 — 最佳实践

- [x] **添加 CHANGELOG.md** — 已创建，记录 v0.1.0 和 v0.2.0 变更历史

- [x] **添加 CI（GitHub Actions）** — 已添加 `.github/workflows/ci.yml`，自动运行 test / clippy / doc 三个 job

- [x] **检查 `cargo package --list`** — 内容正确；`docs/TODO.md` 和 `.github/` 已通过 `exclude` 排除

- [x] **考虑添加 `exclude` 字段** — 已在 `Cargo.toml` 添加 `exclude = ["docs/TODO.md", ".github/"]`
