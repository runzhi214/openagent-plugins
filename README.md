# openagent-plugins

WASM 插件集合，为 [openagent-cli](https://github.com/runzhi214/openagent-go) 提供扩展能力。

## 插件列表

| 插件 | 类型 | 说明 |
|------|------|------|
| `hdspace-models` | `cli:settings` | 从 host keyring 读取华为云 AKSK，调用 TokenHub API 获取模型配置，注入 settings |

## 构建

### 前置条件

- Rust toolchain（含 `rustup`）
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
```

### 编译

`openagent-cli-sdk` 通过 git 依赖自动拉取，无需本地 clone `openagent-go` 仓库。

```bash
make all
```

或手动编译单个插件：

```bash
cargo build --release --target wasm32-unknown-unknown -p hdspace-models
```

产物位于 `build/plugins/` 目录（`make all`）或 `target/wasm32-unknown-unknown/release/` 目录（手动编译）。

### no_std 配置

所有插件均为 `#![no_std]`，workspace `Cargo.toml` 中已配置 `panic = "abort"`（`wasm32-unknown-unknown` 不支持 unwinding）。

---

## hdspace-models

从 host keyring 读取华为云 AKSK，调用 TokenHub API 获取可用模型列表，自动注入到 settings.json 的 `provider` 和 `env` 字段。

### 前置条件

通过 host 的 keyring 预先写入以下凭据（Service 固定为 `openagent`）：

| Key | 必填 | 说明 |
|-----|------|------|
| `HW_ACCESS_KEY` | 是 | 华为云 Access Key |
| `HW_SECRET_KEY` | 是 | 华为云 Secret Key |
| `HW_SECURITY_TOKEN` | 否 | 临时 Security Token |

### 注入内容

插件会在 settings 中注入两块：

**`provider` 块** — 从 API 返回的模型配置：

```json
{
  "provider": {
    "huawei-free": {
      "api_key": "<API 返回的 api_key>",
      "base_url": "<API 返回的 base_url>",
      "models": ["model-id-1", "model-id-2"]
    }
  }
}
```

**`env` 块** — AKSK 凭据注入环境变量：

```json
{
  "env": {
    "HW_ACCESS_KEY": "<ak>",
    "HW_SECRET_KEY": "<sk>",
    "HW_SECURITY_TOKEN": "<token>"
  }
}
```

---

## License

MIT
