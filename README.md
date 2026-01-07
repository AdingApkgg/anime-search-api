# 🎬 AnimeSearch API

基于 Rust + Axum 的在线动漫聚合搜索后端，兼容 [Kazumi](https://github.com/Predidit/Kazumi) 规则格式。

## ✨ 特性

- 🚀 **高性能** - Rust + Tokio 异步运行时
- 📡 **流式响应** - SSE 实时返回搜索结果
- 🔧 **规则驱动** - 兼容 Kazumi 规则格式，直接导入即用
- 🌐 **多平台** - 支持多个动漫资源站点
- 📺 **集数获取** - 自动获取每个结果的集数列表，一键选集播放
- 🖥️ **内置前端** - 自带简洁的搜索页面
- 📺 **Bangumi API** - 完整接入 Bangumi API，支持条目、用户、收藏管理

## 📦 技术栈

| 类别 | 技术 |
|------|------|
| 语言 | Rust 1.92+ |
| 框架 | Axum 0.8 |
| 运行时 | Tokio |
| HTTP 客户端 | Reqwest |
| HTML 解析 | libxml (XPath, 完全兼容 Kazumi) |
| 元数据 | Bangumi API |

## 🚀 快速开始

### 系统依赖

需要安装 libxml2 (用于 XPath 解析，完全兼容 Kazumi 规则):

```bash
# macOS
brew install libxml2

# Ubuntu/Debian
sudo apt install libxml2-dev

# Fedora/RHEL
sudo dnf install libxml2-devel
```

### 编译运行

```bash
cd anime-search-api

# macOS 需要设置 PKG_CONFIG_PATH
export PKG_CONFIG_PATH="/opt/homebrew/opt/libxml2/lib/pkgconfig"

# 开发运行
cargo run

# 生产构建
cargo build --release
./target/release/anime-search-api
```

访问 http://localhost:3000 即可使用搜索页面。

## 📡 API 接口

### 核心接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 搜索页面 |
| POST | `/api` | 搜索动漫 (FormData: `anime=关键词, rules=规则名, episodes=1`) |
| GET | `/info` | API 信息 |
| GET | `/rules` | 获取规则列表 |
| GET | `/update` | 从 KazumiRules 更新规则 |
| GET | `/health` | 健康检查 |

> 💡 设置 `episodes=1` 可获取每个结果的集数列表

### Bangumi API (公开)

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/bangumi/search/{keyword}` | 搜索动漫 |
| GET | `/bangumi/subject/{id}` | 获取条目详情 |
| GET | `/bangumi/calendar` | 每日放送 |

### Bangumi v0 API

**条目相关**

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/bangumi/v0/search` | 条目搜索 | 可选 |
| GET | `/bangumi/v0/subjects/{id}` | 获取条目详情 | 可选 |
| GET | `/bangumi/v0/subjects/{id}/characters` | 获取条目角色 | 可选 |
| GET | `/bangumi/v0/subjects/{id}/persons` | 获取条目制作人员 | 可选 |
| GET | `/bangumi/v0/subjects/{id}/subjects` | 获取关联条目 | 可选 |

**章节相关**

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/bangumi/v0/episodes?subject_id=` | 获取章节列表 | 可选 |
| GET | `/bangumi/v0/episodes/{id}` | 获取章节详情 | 可选 |

**角色/人物**

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/bangumi/v0/characters/{id}` | 获取角色详情 | - |
| POST | `/bangumi/v0/characters/{id}/collect` | 收藏角色 | 🔐 |
| DELETE | `/bangumi/v0/characters/{id}/collect` | 取消收藏 | 🔐 |
| GET | `/bangumi/v0/persons/{id}` | 获取人物详情 | - |
| POST | `/bangumi/v0/persons/{id}/collect` | 收藏人物 | 🔐 |
| DELETE | `/bangumi/v0/persons/{id}/collect` | 取消收藏 | 🔐 |

**用户/收藏**

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/bangumi/v0/users/{username}` | 获取用户信息 | - |
| GET | `/bangumi/v0/me` | 获取当前用户 | 🔐 |
| GET | `/bangumi/v0/users/{username}/collections` | 获取收藏列表 | 🔐 |
| GET | `/bangumi/v0/users/{username}/collections/{id}` | 获取单个收藏 | 🔐 |
| POST | `/bangumi/v0/collections/{subject_id}` | 添加收藏 | 🔐 |
| PATCH | `/bangumi/v0/collections/{subject_id}` | 修改收藏 | 🔐 |
| GET | `/bangumi/v0/collections/{subject_id}/episodes` | 章节收藏 | 🔐 |
| PUT | `/bangumi/v0/collections/episodes/{episode_id}` | 更新章节 | 🔐 |

**目录**

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/bangumi/v0/indices/{id}` | 获取目录详情 | 可选 |
| GET | `/bangumi/v0/indices/{id}/subjects` | 获取目录条目 | 可选 |
| POST | `/bangumi/v0/indices/{id}/collect` | 收藏目录 | 🔐 |
| DELETE | `/bangumi/v0/indices/{id}/collect` | 取消收藏 | 🔐 |

> 🔐 = 需要 `Authorization: Bearer <token>` 请求头
> 
> 获取 Token: https://next.bgm.tv/demo/access-token

### 搜索请求示例

```javascript
const formData = new FormData()
formData.append('anime', '葬送的芙莉莲')
formData.append('rules', 'AGE,MXdm,NT')
formData.append('episodes', '1')  // 可选：获取集数列表

const response = await fetch('/api', {
  method: 'POST',
  body: formData,
})

const reader = response.body.getReader()
// 读取 SSE 流...
```

### 响应格式 (每行一个 JSON)

```json
{"total": 3}
{"progress": {"completed": 1, "total": 3}, "result": {"name": "AGE动漫", "color": "orange", "tags": ["在线"], "items": [{"name": "葬送的芙莉莲", "url": "...", "episodes": [{"name": null, "episodes": [{"name": "01", "url": "..."}, {"name": "02", "url": "..."}]}]}]}}
{"progress": {"completed": 2, "total": 3}}
{"done": true}
```

### 集数响应结构

当 `episodes=1` 时，每个结果项会包含 `episodes` 字段：

```typescript
interface SearchResultItem {
  name: string       // 动漫名称
  url: string        // 详情页链接
  episodes?: EpisodeRoad[]  // 集数列表 (可选)
}

interface EpisodeRoad {
  name?: string      // 播放源名称 (如 "线路1")
  episodes: Episode[]
}

interface Episode {
  name: string       // 集数名称 (如 "01", "第1集")
  url: string        // 播放链接
}
```

### Bangumi API 示例

```javascript
// 搜索动漫
const result = await fetch('/bangumi/search/葬送的芙莉莲').then(r => r.json())

// 获取条目详情
const subject = await fetch('/bangumi/v0/subjects/425249').then(r => r.json())

// 获取每日放送
const calendar = await fetch('/bangumi/calendar').then(r => r.json())

// 需要认证的 API (获取当前用户)
const token = 'your_access_token'
const me = await fetch('/bangumi/v0/me', {
  headers: { 'Authorization': `Bearer ${token}` }
}).then(r => r.json())

// 添加收藏 (type: 1=想看, 2=看过, 3=在看, 4=搁置, 5=抛弃)
await fetch('/bangumi/v0/collections/425249', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    type: 3,  // 在看
    rate: 9,  // 评分 1-10
    comment: '神作！'
  })
})
```

## 📝 规则格式

规则文件放在 `rules/` 目录，每个 `.json` 文件是一个规则。

**完全兼容 [Kazumi 规则](https://github.com/Predidit/KazumiRules)**，可直接下载使用！

### 规则示例

```json
{
  "api": "1",
  "type": "anime",
  "name": "AGE动漫",
  "version": "1.5",
  "muliSources": true,
  "useWebview": true,
  "useNativePlayer": true,
  "userAgent": "",
  "baseURL": "https://www.agedm.io/",
  "searchURL": "https://www.agedm.io/search?query=@keyword",
  "searchList": "section .card",
  "searchName": "h5 a, .card-title a",
  "searchResult": "h5 a, .card-title a",
  "chapterRoads": "",
  "chapterResult": "",
  "color": "orange",
  "tags": ["在线"],
  "magic": false
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 平台名称 |
| `baseURL` | string | 基础 URL |
| `searchURL` | string | 搜索 URL，`@keyword` 为关键词占位符 |
| `searchList` | string | 搜索结果列表 CSS 选择器 |
| `searchName` | string | 结果名称 CSS 选择器 |
| `searchResult` | string | 结果链接 CSS 选择器 |
| `usePost` | bool | 是否使用 POST 请求 |
| `color` | string | 前端显示颜色 (扩展字段) |
| `tags` | array | 平台标签 (扩展字段) |
| `magic` | bool | 是否需要代理 (扩展字段) |

### 导入 Kazumi 规则

从 [KazumiRules](https://github.com/Predidit/KazumiRules) 下载规则文件，放入 `rules/` 目录即可：

```bash
# 下载 Kazumi 规则
curl -o rules/gugu3.json https://raw.githubusercontent.com/Predidit/KazumiRules/main/gugu3.json
```

## 📁 项目结构

```
anime-search-api/
├── Cargo.toml
├── Dockerfile
├── compose.yaml
├── LICENSE
├── README.md
├── rules/              # 规则文件目录 (兼容 Kazumi)
│   ├── AGE.json
│   ├── MXdm.json
│   └── ...
├── static/
│   └── index.html      # 前端页面
└── src/
    ├── main.rs         # 入口 + 路由
    ├── core.rs         # 核心搜索逻辑 (SSE 流)
    ├── engine.rs       # 规则引擎 (XPath 解析)
    ├── rules.rs        # 规则加载器
    ├── types.rs        # 类型定义
    ├── http_client.rs  # HTTP 客户端
    ├── updater.rs      # 规则自动更新
    └── bangumi.rs      # Bangumi API 集成
```

## 🔧 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `PORT` | 3000 | 服务端口 |
| `RUST_LOG` | info | 日志级别 |
| `AUTO_UPDATE` | 0 | 启动时自动更新规则 (1=启用) |
| `BANGUMI_ACCESS_TOKEN` | - | Bangumi API 默认 access token |

### Bangumi 认证说明

需要认证的 Bangumi API (如收藏管理) 支持两种方式提供 token：

1. **客户端传入** - 请求头 `Authorization: Bearer <token>`
2. **服务端默认** - 环境变量 `BANGUMI_ACCESS_TOKEN`

优先使用客户端传入的 token，如未提供则使用服务端配置的默认 token。

获取 token: https://next.bgm.tv/demo/access-token

## 🐳 容器部署

### Podman Compose (推荐)

```bash
# 构建并启动
podman compose up -d --build

# 仅启动 (已构建)
podman compose up -d

# 查看日志
podman compose logs -f

# 停止
podman compose down
```

### Docker Compose

```bash
docker compose up -d --build
```

### 手动构建

```bash
# Podman
podman build -t anime-search-api .
podman run -d -p 3000:3000 -v ./rules:/app/rules:ro --name anime-search-api anime-search-api

# Docker
docker build -t anime-search-api .
docker run -d -p 3000:3000 -v ./rules:/app/rules:ro --name anime-search-api anime-search-api
```

## 🔄 Nginx 反向代理

适配 Nginx 1.29+ / SSL / HTTP/3 / TLSv1.3：

```nginx
server {
    listen 80;
    listen [::]:80;
    server_name anime.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl;
    listen [::]:443 ssl;
    listen 443 quic;
    listen [::]:443 quic;
    server_name anime.example.com;

    # HTTP/3
    http2 on;
    http3 on;
    quic_gso on;
    quic_retry on;
    add_header Alt-Svc 'h3=":443"; ma=86400';

    # SSL/TLS
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    ssl_protocols TLSv1.3;
    ssl_prefer_server_ciphers off;
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:10m;
    ssl_session_tickets off;

    # OCSP Stapling
    ssl_stapling on;
    ssl_stapling_verify on;
    ssl_trusted_certificate /path/to/chain.pem;

    # Security Headers
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # SSE 流式响应
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 86400s;
        chunked_transfer_encoding on;
    }
}
```

> ⚠️ `proxy_buffering off` 确保 SSE 流式响应正常工作

## 🙏 致谢

- [Kazumi](https://github.com/Predidit/Kazumi) - 规则格式参考
- [KazumiRules](https://github.com/Predidit/KazumiRules) - 规则仓库

## 📄 License

MIT
