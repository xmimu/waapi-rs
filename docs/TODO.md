# waapi-rs 发布待办清单

## P0 — 发布阻塞项

- [ ] **解决 `wamp_async` git 依赖（方案 B：以新名字发布 fork）**
  crates.io 不允许 git 依赖。采用方案 B —— 将 fork 以新 crate 名发布到 crates.io，解除发布阻塞。
  1. [ ] 精简 fork：在 `xmimu/wamp_async` 的 dev 分支上，去掉 `waapi_client.rs`、WAAPI examples、`WAAPI_CLIENT_GUIDE.md` 等与核心改动无关的文件
  2. [ ] 修改 fork 的 `Cargo.toml`：`name = "wamp-async-waapi"`，添加 `license`、`description`、`repository` 等元数据，确保 `rmp-serde` 版本兼容
  3. [ ] 发布 fork 到 crates.io：`cargo publish`
  4. [ ] 修改 waapi-rs 的 `Cargo.toml`：`wamp_async` 改为 `wamp-async-waapi = "0.3.2"`（或对应版本）
  5. [ ] 更新 waapi-rs 代码中的 `use wamp_async::` 路径为 `use wamp_async_waapi::`（取决于 crate 名与 lib name 的映射）
  
  > 建议：可同时向上游提交 PR（方案 A），PR 合并后再切回官方版本

- [x] **添加 `license` 字段** — 已添加 `license = "MIT"`

- [x] **添加 LICENSE 文件** — 已创建 MIT LICENSE

- [x] **添加 `description` 字段** — 已添加

## P1 — 功能与改进

- [ ] **subscribe 支持传 options 参数**
  当前 `subscribe` / `subscribe_with_callback` 不支持传入 options，需要增加带 options 的重载（如 `subscribe_with_options(topic, options)` / `subscribe_with_callback_and_options(topic, options, callback)`），以支持 WAAPI 订阅时的过滤、返回字段等选项

## P1 — 强烈建议

- [ ] **补充 `Cargo.toml` 中的 `authors` 格式**
  当前为 `"xmimu 1101588023@qq.com"`，标准格式应为 `"xmimu <1101588023@qq.com>"`

- [ ] **README 安装说明更新**
  发布后需将安装方式从 git 依赖改为 crates.io 版本号依赖，如：
  ```toml
  [dependencies]
  waapi-rs = "0.1.0"
  ```

- [ ] **确认公开 API 的文档注释完整**
  运行 `cargo doc --no-deps` 检查生成文档，确保所有 pub 类型和方法都有文档

- [ ] **运行 `cargo clippy` 修复所有 lint 警告**

- [ ] **运行 `cargo test` 确保测试通过**

- [ ] **运行 `cargo publish --dry-run` 模拟发布，确认无报错**

## P2 — 最佳实践

- [ ] **添加 CHANGELOG.md** — 记录版本变更历史

- [ ] **添加 CI（GitHub Actions）** — 自动运行 clippy / test / doc

- [ ] **检查 `cargo package --list`** — 确认打包内容正确，无多余文件

- [ ] **考虑添加 `exclude` 字段** — 排除 `docs/`、`examples/` 等非必要目录（如果不想包含在发布包中）
