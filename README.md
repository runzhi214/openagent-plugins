# openagent-plugins

WASM 插件集合，为 [openagent-cli](https://github.com/runzhi214/openagent-go) 提供扩展能力。

## 插件列表

| 插件 | 类型 | 说明                                                                            |
|------|------|---------------------------------------------------------------------------------|
| `hdspace-models` | `cli:settings` | 从 host keyring 读取华为云 AKSK，调用 TokenHub API 获取模型配置，注入 settings  |
| `hdspace-renew` | `agent:observers` | 当模型返回 HTTP 认证错误时，重新从 keyring 读取 AKSK，调用 TokenHub API 刷新所有模型配置 |
| `hdspace-envsync` | `agent:tools` (scheduled) | 每 5 分钟从 keyring 读取 HW_ACCESS_KEY/HW_SECRET_KEY/HW_SECURITY_TOKEN 同步到宿主进程环境变量 |

## 构建

### 前置条件

- Rust toolchain（含 `rustup`）
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
```

### 编译

`openagent-pdk` 通过 git 依赖自动拉取，无需本地 clone `openagent-go` 仓库。

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

### Docker 编译

无需本地安装 Rust 工具链，使用 Docker 即可编译：

```bash
docker build --output type=local,dest=build/plugins .
```

或通过 Makefile：

```bash
make docker
```

产物直接输出到 `build/plugins/` 目录。

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

## hdspace-renew

`agent:observers` 插件，监听 `model.call.leave` 阶段。当模型调用返回 HTTP 认证错误时（由预定义触发关键词检测），自动从 host keyring 重新读取华为云 AKSK 凭据，调用 TokenHub API 获取最新的模型配置（api_key / base_url / 模型列表），然后逐个调用宿主的 `runtime_set_model_config` 方法更新每个模型的参数。

### 工作流程

1. 检测到 `model.call.leave` 阶段的错误包含预定义的触发关键词
2. 从 keyring 读取 `HW_ACCESS_KEY`、`HW_SECRET_KEY`（及可选的 `HW_SECURITY_TOKEN`）
3. 使用 SDK-HMAC-SHA256 签名调用 TokenHub API 获取最新模型配置
4. 遍历返回的模型列表，逐个调用 `runtime_set_model_config` 更新 `api_key` 和 `base_url`
5. 返回 `continue`，更新后的配置在后续调用中生效

### Keyring 凭据

与 `hdspace-models` 共用同一组 keyring 凭据（Service 为 `openagent`）：

| Key | 必填 | 说明 |
|-----|------|------|
| `HW_ACCESS_KEY` | 是 | 华为云 Access Key |
| `HW_SECRET_KEY` | 是 | 华为云 Secret Key |
| `HW_SECURITY_TOKEN` | 否 | 临时 Security Token（临时 AKSK 场景） |

---

## hdspace-envsync

定时作业插件，声明一个每 5 分钟执行的 cron job（`*/5 * * * *`）。到点时从 host keyring 读取华为云 AKSK 凭据，同步到宿主进程的环境变量中，使下游消费者（iac-server、terraform 子进程、SDK-HMAC-SHA256 签名等）无需重启即可拿到最新凭据。

### 工作流程

1. 宿主 scheduler 每 5 分钟触发插件的 `run_scheduled` 导出
2. 从 keyring（service `openagent`）读取 `HW_ACCESS_KEY`、`HW_SECRET_KEY`
3. 调用 `host::env_set` 写入宿主进程环境变量
4. `HW_SECURITY_TOKEN` 为可选——存在则设，不存在则 `env_unset` 清除防残留

### Keyring 凭据

与 `hdspace-models` / `hdspace-renew` 共用同一组 keyring 凭据（Service 为 `openagent`）。

### 部署

`agent:*` 插件需放置在 `~/.openagent/profile/plugins/`（agent 插件系统加载目录），与 `cli:*` 插件的 `~/.openagent/plugins/` 不同。

---

## License

MIT
