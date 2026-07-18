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
rustup target add wasm32-unknown-unknown
make all
```

产物位于 `build/plugins/` 目录。

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

支持通过环境变量 `HW_MODELS_DOMAIN` 覆盖 API 域名（默认 `devstation.myhuaweicloud.com`）。

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

## extended-settings

从 host keyring 读取自定义 provider 凭证，注入到 settings.json。

### 前置条件

| Key | 必填 | 说明 |
|-----|------|------|
| `my_provider_api_key` | 是 | provider 的 API Key |
| `my_provider_base_url` | 是 | provider 的 Base URL |
| `my_provider_models` | 否 | 模型 ID 列表，逗号分隔 |

### 注入内容

```json
{
  "provider": {
    "my_provider": {
      "api_key": "<keyring 中的 api_key>",
      "base_url": "<keyring 中的 base_url>",
      "models": ["model-a", "model-b"]
    }
  },
  "env": {
    "MY_PROVIDER_API_KEY": "<api_key>"
  }
}
```

---

## License

MIT
