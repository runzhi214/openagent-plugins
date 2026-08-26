# 接入外部 Embedding 模型

openagent-cli 默认纯 Go 构建（CGO_ENABLED=0），无内嵌模型。知识库 CRUD + 关键词召回
开箱即用；若要启用语义向量召回，需在 settings.json 里配置 `embedding` 段，
指向一个 OpenAI 兼容 `/embeddings` 端点（外部 provider）。

## 配置

编辑 `~/.openagent/settings.json`（或 `$OPENAGENT_CLI_CONFIG` 指向的路径）：

```json
{
  "embedding": {
    "provider": "openai",
    "base_url": "https://your-host/v1",
    "api_key": "sk-xxx",
    "model": "your-embedding-model-id"
  }
}
```

| 字段       | 说明                                                                 |
|------------|----------------------------------------------------------------------|
| `provider` | 非空即走外部后端；填 `"openai"`（OpenAI 兼容端点通用：OpenAI / Ollama / Jina / Cohere / 本地网关） |
| `base_url` | 不带末尾 `/embeddings`，代码自动拼接                                  |
| `api_key`  | Bearer token；为空则不发 `Authorization` 头（本地 Ollama 可留空）      |
| `model`    | embedding 模型 ID，如 `text-embedding-3-small`                        |

## 生效条件

- `--embedder` 需为 `on`（默认就是 on）；为 off 则整个 embedding 关闭，降级 keyword 召回
- `provider` 非空 → 启用语义向量召回（外部 embedding）；`provider` 为空 → 降级为关键词 LIKE 匹配（无向量）
- 向量维度从首次 Embed 响应自动缓存

## 协议要求

外部端点必须实现 OpenAI `/embeddings` 协议格式：

- `POST {base_url}/embeddings`
- 请求体：`{"model": "...", "input": "text"}`
- 响应体：`{"data": [{"embedding": [float, ...]}]}`
- 鉴权：`Authorization: Bearer {api_key}`（api_key 非空时）

若端点不是 OpenAI 兼容格式（自有签名 / 路径），需在 `embedder/` 下新写一个实现
`openagent.Embedder` 接口（`Embed(ctx, text) ([]float64, error)` + `Dimensions() int`）的 backend。

## 数据流

```
settings.json embedding 段
  → cmd/cli/server/shared.go buildMemory()
  → embedder/openai.New(base_url, api_key, model)
  → memory/sqlite.WithEmbedder()
  → Store 时 indexEmbedding 算向量写 knowledge_vectors 表
  → Recall 时 knowledgeVectorRecall 做 cosine 召回（vector-first，keyword 兜底）
```

## 关闭语义召回

`--embedder=off` 或 settings.json：

```json
{ "capabilities": { "embedder": false } }
```
