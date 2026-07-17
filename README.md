# openagent-plugins

WASM 插件集合，为 [openagent-cli](https://github.com/runzhi214/openagent-go) 提供扩展能力。

## 插件列表

| 插件 | 类型 | 说明 |
|------|------|------|
| `hdspace-models` | `cli:settings` | 从 host keyring 读取华为云 AKSK，调用 TokenHub API 获取模型配置，注入 settings |
| `extended-settings` | `cli:settings` | 从 host keyring 读取 provider 凭证，注入 settings |
| `stats-cmd` | `cli:commands` | 新增 `stats` 命令，查看插件状态 |
| `telemetry` | `cli:observers` | 生命周期事件日志 |

## 构建

```bash
# 需要 wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# 构建全部插件
make all

# 构建单个插件
cargo build --release --target wasm32-unknown-unknown -p hdspace-models
```

产物位于 `build/plugins/` 目录。

## hdspace-models

读取 host 提供的以下 keyring 凭据：

| Key | Service | 说明 |
|-----|---------|------|
| `HW_ACCESS_KEY` | `openagent` | 华为云 AK |
| `HW_SECRET_KEY` | `openagent` | 华为云 SK |
| `HW_SECURITY_TOKEN` | `openagent` | Security Token（可选） |

通过华为云 SDK-HMAC-SHA256 签名调用 `/open-api-public/v1/tokenhub-configs` API，获取模型列表后注入到 settings 的 `provider` 和 `env` 字段。

支持 `HW_MODELS_DOMAIN` 环境变量覆盖默认域名（默认为 `devstation.myhuaweicloud.com`）。

### 依赖

依赖 `openagent-cli-sdk`，路径为 `../../openagent-go/cmd/cli/sdk/rust`。SDK 需要新增以下 host 函数：

- `get_env(key) -> value` — 读取 host 环境变量
- `get_time_utc() -> "YYYYMMDDTHHMMSSZ"` — 获取当前 UTC 时间

## License

MIT
