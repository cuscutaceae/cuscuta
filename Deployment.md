简体中文 | [English](Deployment.en.md)

# 构建与部署

## 前置依赖

- PostgreSQL 15+
- Redis 7+
- （若使用 Helm 部署）已安装 [KEDA](https://keda.sh) 的 Kubernetes 集群

预构建镜像位于 `ghcr.io/cuscutaceae`，所有组件均支持 amd64 架构。

---

## 快速开始

### Helm


```shell
# 1. 复制最小配置文件并填写必填项
cp helm/values.default.yaml my-values.yaml
# 编辑 my-values.yaml，填入 postgresql.url、redis.url、chilo.constants、github.*、api.*

# 2. 安装
helm install cuscuta oci://ghcr.io/cuscutaceae/charts/cuscuta --version 0.1.0 -f my-values.yaml

# 3. 验证集群状态
cuscutactl --mode kubernetes doctor
```

Helm chart 详情见 [helm/values.yaml](helm/values.yaml)，最小配置模板见 [helm/values.default.yaml](helm/values.default.yaml)。

---

### Docker Compose

适用于本地测试，内置了基础 PostgreSQL、Redis 服务，并默认开启 mock 服务：

```shell
cp docker-compose-local.yaml docker-compose.override.yaml
# 编辑 docker-compose.override.yaml，填入 GITHUB_*_TOKEN 和 chilo 常量
docker compose up -d
```

> [!IMPORTANT]
> Docker Compose 不支持 KEDA 自动伸缩，entry 和 worker 均为固定副本数且不可动态扩容，推荐使用 Helm。

---

## 构建

> [!TIP]
>
> cuscuta具有 amd64 架构下的预构建镜像，其位于 `ghcr.io/cuscutaceae`，若无特殊需求，可直接使用ghcr.io的托管镜像

### Docker或nerdctl（推荐）

```shell
git clone https://github.com/cuscutaceae/cuscuta
cd cuscuta
docker compose build
# 若您使用 containerd，则可以考虑使用 nerdctl进行构建，下文同理
# nerdctl compose build
```

单独构建某个镜像：

```shell
docker build -t cuscuta-worker:local -f cuscuta-worker/Dockerfile  .
docker build -t cuscuta-entry:local -f cuscuta-entry/Dockerfile  .
docker build -t cuscuta-chilo:local -f cuscuta-chilo/Dockerfile  .
docker build -t cuscuta-mock:local  -f cuscuta-mock/Dockerfile   .
```

### 本地编译（不推荐）

> [!NOTE]
> 
> `cuscuta-chilo` 依赖 [`chilo`](https://github.com/cuscutaceae/chilo)，所以需要 `clang`、`cmake`、`pkg-config` 作为编译依赖。
> 若您处于 Windows 环境下，则建议在`Visual Studio Install`安装特定组件后，在 Visual Studio 的 Developer Command Prompt 环境下进行构建。

```shell
cargo build --release -p cuscuta-entry
cargo build --release -p cuscuta-worker
cargo build --release -p cuscuta-chilo
```

---

## Helm Chart 参考

### 组件概览

| 组件 | 类型 | 默认副本数 | 伸缩方式 |
|------|------|-----------|----------|
| `cuscuta-chilo` | Deployment | 1 | 手动 |
| `cuscuta-entry` | Deployment | 1 | **不可伸缩**（单例服务） |
| `cuscuta-worker` | Deployment + ScaledObject | 1 | KEDA（PostgreSQL 触发器） |
| `cuscuta-mock` | Deployment | 1（默认关闭） | 手动 |
| 数据库迁移 | Job（Helm hook） | — | pre-install / pre-upgrade |

### 关键配置项

完整列表见 [helm/values.yaml](helm/values.yaml)。

| 配置项 | 说明 |
|--------|------|
| `postgresql.url` | PostgreSQL 连接字符串 |
| `redis.url` | Redis 连接字符串 |
| `chilo.constants.binC1` | 常量 C1（十六进制） |
| `chilo.constants.binC2` | 常量 C2（十六进制） |
| `chilo.constants.binLoginC31` | 常量 login-C31（十六进制） |
| `chilo.constants.binLoginC32` | 常量 login-C32（十六进制） |
| `github.bundleRepository` | Bundle 数据所在的 GitHub 仓库 |
| `github.bundlePath` | 仓库中 bundle JSON 的路径 |
| `github.bundleToken` | 访问 bundle 仓库的 GitHub PAT |
| `github.songRepository` | 曲目列表所在的 GitHub 仓库 |
| `github.songPath` | 仓库中曲目列表的路径 |
| `github.songToken` | 访问曲目仓库的 GitHub PAT |
| `api.login` | 目标登录 API 地址 |
| `api.listFriends` | 目标好友列表 API 地址 |
| `api.addFriends` | 目标添加好友 API 地址 |
| `api.deleteFriends` | 目标删除好友 API 地址 |
| `api.getRank` | 目标排行榜 API 地址 |
| `worker.keda.enabled` | 是否启用 KEDA 自动伸缩 |
| `worker.keda.maxReplicaCount` | worker 最大副本数 |
| `mock.enabled` | 是否启用 mock 服务（生产环境应关闭） |

---

## 环境变量参考

cuscuta 所有组件均通过环境变量注入配置。

### 共用变量

| 变量 | 使用者 |
|------|--------|
| `RUST_LOG` | 全部 |
| `ACCOUNTS_SQL_ADDR` | entry, worker |
| `REDIS_ADDR` | entry, worker |
| `REDIS_STREAM_REFRESH_TTL` | entry, worker |
| `ETA_ENABLE` | entry, worker |
| `ETA_SEARCH_LIMIT` | entry, worker |
| `ETA_RECORD_TRIM` | entry |
| `GITHUB_BUNDLE_REPOSITORY` | entry, worker |
| `GITHUB_BUNDLE_PATH` | entry, worker |
| `GITHUB_BUNDLE_TOKEN` | entry, worker |
| `GITHUB_SONG_REPOSITORY` | entry, worker |
| `GITHUB_SONG_PATH` | entry, worker |
| `GITHUB_SONG_TOKEN` | entry, worker |

### Worker 专用变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `WORKER_MAX_JOBS` | 8 | 每个 worker 最大并发任务数 |
| `WORKER_MAX_RETRIES` | 30 | API 调用最大重试次数 |
| `WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS` | 10 | 指数退避初始等待时间（毫秒） |
| `WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER` | 2 | 指数退避乘数 |
| `WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS` | 500 | 指数退避最大等待时间（毫秒） |
| `WORKER_ACCOUNT_LEASE_TIME_SECS` | 120 | 账号租约时长（秒） |
| `WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS` | 30 | 租约续期间隔（秒） |
| `WORKER_JOB_MAX_WORK_TIME_SECS` | 1200 | 任务的最长运行时间（秒） |
| `ETA_ENABLE` | true | 是否启用 ETA 预估 |
| `API_CHILO` | — | chilo 服务地址 |
| `API_LOGIN` | — | 登录 API 地址 |
| `API_LIST_FRIENDS` | — | 好友列表 API 地址 |
| `API_ADD_FRIENDS` | — | 添加好友 API 地址 |
| `API_DELETE_FRIENDS` | — | 删除好友 API 地址 |
| `API_GET_RANK` | — | 排行榜 API 地址 |

### Chilo 专用变量

| 变量 | 说明 |
|------|------|
| `BIN_C1` | 常量 C1（十六进制，32字节长） |
| `BIN_C2` | 常量 C2（十六进制，32字节长） |
| `BIN_LOGIN_C31` | 常量 login-C31（十六进制，10字节长） |
| `BIN_LOGIN_C32` | 常量 login-C32（十六进制，10字节长） |

---

## 集群管理

您可以使用 [cuscutactl](cuscutactl/README.md) 进行简易的管理操作：

> [!TIP]
>
> `account.txt`的格式为`account_email:password`，数据之间使用换行符分割

```shell
# 健康检查（Kubernetes 模式）
# cuscutactl --mode kubernetes --kube-namespace cuscuta doctor

# 健康检查（Legacy 模式）
cuscutactl --mode legacy --postgresql-url "..." --redis-url "..." doctor

# 查看账号概况
cuscutactl --mode legacy --postgresql-url "..." accounts status --max-count 20

# 批量导入账号
cat accounts.txt | cuscutactl --mode legacy --postgresql-url "..." accounts row add --stdin

# 查看任务结果
cuscutactl --mode legacy --redis-url "..." jobs result --code 123456789 --print-detail
```
