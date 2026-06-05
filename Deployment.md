## cuscuta的构建与部署

### 注意！

**cuscuta目前处于不稳定状态，部署带来的后果由使用者自行承担！**

### 构建

> [!NOTE]
>
> 本仓库在ghcr.io上具有workflow自动构建的amd64架构镜像，如果构建过程太过繁琐（以及在信任我的前提下），可以直接使用ghcr.io上的镜像而无需构建

cuscuta使用Dockerfile进行构建，cuscuta包含4个镜像（`cuscuta-entry`, `cuscuta-worker`, `cuscuta-chilo`, `cuscuta-mock`），其中`cuscuta-mock`的构建为可选的

#### docker/nerdctl本地构建

```shell
git clone https://github.com/cuscutaceae/cuscuta
cd cuscuta
docker-compose create
# 或 nerdctl compose create
```

#### 构建为可执行文件（并不推荐）

如果只是想构建单个组件，可以使用`docker build -t=cuscuta-worker:local -f cuscuta-worker/Dockerfile .`这样的命令来构建单个镜像，如果您实在不想使用Docker或者nerdctl，您也可以在安装Rust编译工具链后自行使用`cargo build --release`构建单个可执行应用

> [!NOTE]
> 
> 其中，由于`cuscuta-chilo`依赖[chilo](https://github.com/cuscutaceae/chilo)，所以需要安装`clang`, `cmake`, `pkg-config`作为构建依赖，如果您处于Windows环境下，可以考虑在Visual Studio安装相应组件后，使用`Developer Command Prompt for VS xxxx`命令行进行构建

> [!IMPORTANT]
>
> 出于某些原因，cuscuta并不包含对应资源的token、目标API地址，以及chilo所需的关键常量，若您持有这些资源，请自行配置

### 部署

cuscuta有两种部署方式：本地测试用的docker-compose方式，和部署在Kubernetes集群上的helm方式；由于cuscuta的启动需要配置环境变量作为参数 ，并且docker-compose环境缺乏KEDA(Kubernetes-based Event-Driven Autoscaler)支持的动态扩容，为统一化管理，推荐使用helm部署方式；docker-compose部署方式较为便捷，适合本地快速验证

cuscuta统一使用环境变量注入配置，具体如下表：

| 变量 | 含义 |
| ---- | ---- |
|`WORKER_MAX_JOBS`|worker的最大任务数|
|`WORKER_MAX_RETRIES`|worker调用某些API时的最大尝试次数|
|`WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS`|worker调用某些API时，指数退避等待的初始等待时间，毫秒计|
|`WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER`|worker调用某些API时，指数退避等待的退避因子|
|`WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS`|worker调用某些API时，指数退避等待的最大等待时间，毫秒计|
|`WORKER_ACCOUNT_LEASE_TIME_SECS`|worker更新`lease_time`时的租期，秒计|
|`WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS`|worker定期续租的间隔，秒计|
|`REDIS_STREAM_REFRESH_TTL`|Redis刷新队列和结果列表时的目标存活时间|
|`GITHUB_BUNDLE_REPOSITORY`|GitHub上Bundle资源的仓库路径|
|`GITHUB_BUNDLE_PATH`|GitHub上Bundle资源的仓库地址|
|`GITHUB_BUNDLE_TOKEN`|GitHub上Bundle资源的访问Token|
|`GITHUB_SONG_REPOSITORY`|GitHub上SongList资源的仓库路径|
|`GITHUB_SONG_PATH`|GitHub上SongList资源的仓库地址|
|`GITHUB_SONG_TOKEN`|GitHub上SongList资源的访问Token|
|`REDIS_ADDR`|Redis数据库的URL|
|`ACCOUNTS_SQL_ADDR`|储存账户数据的PostgreSQL数据库的URL|
|`API_CHILO`|cuscuta-chilo的Api地址|
|`API_LOGIN`|登录API|
|`API_LIST_FRIENDS`|好友列表API|
|`API_ADD_FRIENDS`|添加好友API|
|`API_DELETE_FRIENDS`|删除好友API|
|`API_GET_RANK`|获取排行榜API|
|`BIN_C1`|常量`C1`|
|`BIN_C2`|常量`C2`|
|`BIN_LOGIN_C31`|常量`LOGIN_C31`|
|`BIN_LOGIN_C32`|常量`LOGIN_C32`|

> [!NOTE]
>
> 以上为镜像所需的原始环境变量值，对于helm包的value对应，另见[TODO]()

#### docker-compose

使用docker-compose部署时，建议使用本地构建方式，您可以先根据上文使用`docker-compose create`提前构建，也可以使用`docker-compose up -d`直接启动，缺失的构建将会在启动时进行

cuscuta的docker-compose.yaml默认使用本地构建的镜像，您也可以使用在ghcr.io上的镜像在docker-compose环境下部署cuscuta，对于这种情况，请自行修改docker-compose.yaml：

```yaml
test-worker:
    # 如需使用网络镜像。请将下面这一行解除注释，如有必要，也可以更改tag
    # image: ghcr.io/cuscutaceae/cuscuta-worker:latest
    # 然后删除或注释下面三行
    build:
      context: .
      dockerfile: ./cuscuta-worker/Dockerfile
```

> [!IMPORTANT]
>
> 为方便快速原型部署，docker-compose.yaml中配置了基本的PostgreSQL、Redis和cuscuta-mock服务，注意，helm部署方法**不包含**这三个服务，请根据实际情况自行配置，详情请见下文

#### helm

TODO

