[简体中文](Deployment.md) | English

# Building & Deploying cuscuta

## Prerequisites

- PostgreSQL 15+
- Redis 7+
- (Helm deployment only) Kubernetes cluster with [KEDA](https://keda.sh) installed

Pre-built images are available at `ghcr.io/cuscutaceae`. All components support amd64.

---

## Quick Start

### Helm

```shell
# 1. Copy the minimal config and fill in the blanks
cp helm/values.default.yaml my-values.yaml
# Edit my-values.yaml: postgresql.url, redis.url, chilo.constants.binC2, dataSource.* and api.*

# 2. Install
helm install cuscuta oci://ghcr.io/cuscutaceae/charts/cuscuta --version 0.1.3 -f my-values.yaml

# 3. Verify cluster health
cuscutactl --mode kubernetes doctor
```

See [helm/values.yaml](helm/values.yaml) for the full reference, and
[helm/values.default.yaml](helm/values.default.yaml) for the minimal template.

---

### Docker Compose

For local testing. Includes basic PostgreSQL and Redis services, with the mock
service enabled by default.

```shell
cp docker-compose-local.yaml docker-compose.override.yaml
# Edit docker-compose.override.yaml: data source URLs, API endpoints and chilo key
docker compose up -d
```

> [!IMPORTANT]
> Docker Compose does not support KEDA autoscaling. Both entry and worker
> run at fixed replicas and cannot scale dynamically. Use Helm for production.

---

## Build

> [!TIP]
>
> Pre-built amd64 images are available at `ghcr.io/cuscutaceae`. If you have
> no special requirements, use the hosted images directly.

### Docker or nerdctl (recommended)

```shell
git clone https://github.com/cuscutaceae/cuscuta
cd cuscuta
docker compose build
# If you use containerd, consider nerdctl instead:
# nerdctl compose build
```

Build a single image:

```shell
docker build -t cuscuta-worker:local -f cuscuta-worker/Dockerfile  .
docker build -t cuscuta-entry:local -f cuscuta-entry/Dockerfile  .
docker build -t cuscuta-chilo:local -f cuscuta-chilo/Dockerfile  .
docker build -t cuscuta-mock:local  -f cuscuta-mock/Dockerfile   .
```

### Native build (not recommended)

> [!NOTE]
>
> `cuscuta-chilo` depends on [`chilo`](https://github.com/cuscutaceae/chilo),
> which requires `clang`, `cmake`, and `pkg-config` as build dependencies.
> On Windows, install the required components via Visual Studio Installer and
> build inside a Developer Command Prompt.

```shell
cargo build --release -p cuscuta-entry
cargo build --release -p cuscuta-worker
cargo build --release -p cuscuta-chilo
```

---

## Helm Chart Reference

### Component overview

| Component | Kind | Default replicas | Scaling |
|-----------|------|------------------|---------|
| `cuscuta-chilo` | Deployment | 1 | manual |
| `cuscuta-entry` | Deployment | 1 | **do not scale** (singleton) |
| `cuscuta-worker` | Deployment + ScaledObject | 1 | KEDA (PostgreSQL trigger) |
| `cuscuta-mock` | Deployment | 1 (disabled) | manual |
| Database migration | Job (Helm hook) | — | pre-install / pre-upgrade |

### Key values

See [helm/values.yaml](helm/values.yaml) for the complete list.

| Value | Description |
|--------|-------------|
| `postgresql.url` | PostgreSQL connection string |
| `postgresql.secret.enabled` | Read PostgreSQL URL from an external Secret |
| `postgresql.secret.name` | External Secret name |
| `postgresql.secret.key` | External Secret key for PostgreSQL URL |
| `redis.url` | Redis connection string |
| `redis.secret.enabled` | Read Redis URL from an external Secret |
| `redis.secret.name` | External Secret name |
| `redis.secret.key` | External Secret key for Redis URL |
| `rustLog` | Log level for all components (env `RUST_LOG`) |
| `redisStreamRefreshTtl` | Redis stream TTL shared by entry and worker, in seconds (env `REDIS_STREAM_REFRESH_TTL`) |
| `chilo.constants.useOnlineKey` | Fetch the key online (env `USE_ONLINE_KEY`) |
| `chilo.constants.binC2` | Offline key C2, hex (env `BIN_C2`) |
| `dataSource.base.appVersion.useOnlineVersion` | Fetch the app version online (env `RESOURCES_APP_VERSION_USE_ONLINE`) |
| `dataSource.base.appVersion.url` | App version JSON URL (env `RESOURCES_APP_VERSION_URL`) |
| `dataSource.base.appVersion.versionDefault` | Fallback app version when offline (env `RESOURCES_APP_VERSION_DEFAULT`) |
| `dataSource.base.songUrl` | Song list JSON URL (env `RESOURCES_SONG_URL`) |
| `dataSource.scirpophaga.url` | scirpophaga data JSON URL (env `SCIRPOPHAGA_URL`) |
| `dataSource.update.dataUpdatePeriod` | Data refresh period in seconds (env `RESOURCE_UPDATE_PERIOD`) |
| `dataSource.update.dataUpdateRetries` | Max data-fetch retries (env `RESOURCE_UPDATE_RETRIES`) |
| `api.prefix.useOnlinePrefix` | Fetch API path prefixes online (env `USE_ONLINE_PREFIX`) |
| `api.prefix.auth` | Auth API path prefix (env `API_PREFIX_AUTH`) |
| `api.prefix.common` | Common API path prefix (env `API_PREFIX_COMMON`) |
| `api.path.login` | Login API path (env `API_ENDPOINT_LOGIN`) |
| `api.path.listFriends` | Friend list API path (env `API_ENDPOINT_LIST_FRIENDS`) |
| `api.path.addFriends` | Add-friend API path (env `API_ENDPOINT_ADD_FRIENDS`) |
| `api.path.deleteFriends` | Delete-friend API path (env `API_ENDPOINT_DELETE_FRIENDS`) |
| `api.path.getRank` | Rank API path (env `API_ENDPOINT_GET_RANK`) |
| `api.path.notification` | Notification API path (env `API_ENDPOINT_NOTIFICATION`) |
| `api.path.compose` | Compose aggregate API path (env `API_ENDPOINT_COMPOSE_AGGREGATE`) |
| `entry.status.enabled` | Enable the `/v1/status` endpoint (env `STAT_ENABLE`) |
| `worker.maxJobs` | Max concurrent jobs per worker (env `WORKER_MAX_JOBS`) |
| `worker.maxRetries` | Max API retry attempts (env `WORKER_MAX_RETRIES`) |
| `worker.exponentialBackoffBaseMillis` | Exponential backoff base, ms (env `WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS`) |
| `worker.exponentialBackoffMultiplier` | Exponential backoff multiplier (env `WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER`) |
| `worker.exponentialBackoffMaxDelayMillis` | Max backoff delay, ms (env `WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS`) |
| `worker.accountLeaseTimeSecs` | Account lease duration, s (env `WORKER_ACCOUNT_LEASE_TIME_SECS`) |
| `worker.accountLeaseTimeRefreshGapSecs` | Lease refresh interval, s (env `WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS`) |
| `worker.jobMaxWorkTimeSecs` | Maximum working time for a job, s (env `WORKER_JOB_MAX_WORK_TIME_SECS`) |
| `worker.emptyFriendsDelayTimeSecs` | Rate limiting delay time, s (env `WORKER_EMPTY_FRIENDS_DELAY_TIME_SECS`) |
| `worker.keda.enabled` | Enable KEDA autoscaling |
| `worker.keda.maxReplicaCount` | Maximum worker replicas |
| `mock.enabled` | Enable mock service (disable in production) |

---

## Environment Variables

All cuscuta components receive configuration through environment variables.

### Common infrastructure

| Variable | Used by | Description |
|----------|---------|-------------|
| `RUST_LOG` | all | Log level |
| `ACCOUNTS_SQL_ADDR` | entry, worker | PostgreSQL connection string |
| `REDIS_ADDR` | entry, worker | Redis connection string |
| `REDIS_STREAM_REFRESH_TTL` | entry, worker | Redis stream TTL (seconds) |

### Data sync

| Variable | Used by | Description |
|----------|---------|-------------|
| `RESOURCE_UPDATE_PERIOD` | entry, worker, chilo | Data refresh period (seconds, default 30) |
| `RESOURCE_UPDATE_RETRIES` | entry, worker, chilo | Max data-fetch retries (default 5) |
| `RESOURCES_SONG_URL` | entry, worker | Song list JSON URL |
| `RESOURCES_APP_VERSION_USE_ONLINE` | worker | Fetch the app version online (default true) |
| `RESOURCES_APP_VERSION_URL` | worker | App version JSON URL (used when online) |
| `RESOURCES_APP_VERSION_DEFAULT` | worker | Fallback app version when offline |

### Target API (worker)

| Variable | Description |
|----------|-------------|
| `USE_ONLINE_PREFIX` | Fetch API path prefixes online (default true) |
| `SCIRPOPHAGA_URL` | scirpophaga data JSON URL (provides API path prefixes) |
| `API_PREFIX_AUTH` | Auth API path prefix (used when offline) |
| `API_PREFIX_COMMON` | Common API path prefix (used when offline) |
| `API_CHILO` | chilo service URL |
| `API_ENDPOINT_LOGIN` | Login API path |
| `API_ENDPOINT_LIST_FRIENDS` | Friend list API path |
| `API_ENDPOINT_ADD_FRIENDS` | Add-friend API path |
| `API_ENDPOINT_DELETE_FRIENDS` | Delete-friend API path |
| `API_ENDPOINT_GET_RANK` | Rank API path |
| `API_ENDPOINT_NOTIFICATION` | Notification API path |
| `API_ENDPOINT_COMPOSE_AGGREGATE` | Compose aggregate API path |

### Entry-only

| Variable | Description |
|----------|-------------|
| `STAT_ENABLE` | Enable the `/v1/status` endpoint |

### Worker tuning

| Variable | Description |
|----------|-------------|
| `WORKER_MAX_JOBS` | Max concurrent jobs per worker |
| `WORKER_MAX_RETRIES` | Max API retry attempts |
| `WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS` | Exponential backoff base (ms) |
| `WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER` | Exponential backoff multiplier |
| `WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS` | Max backoff delay (ms) |
| `WORKER_ACCOUNT_LEASE_TIME_SECS` | Account lease duration (s) |
| `WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS` | Lease refresh interval (s) |
| `WORKER_JOB_MAX_WORK_TIME_SECS` | Maximum working time for a job (s) |
| `WORKER_EMPTY_FRIENDS_DELAY_TIME_SECS` | Rate limiting delay time (s) |

### Chilo-only

| Variable | Description |
|----------|-------------|
| `USE_ONLINE_KEY` | Fetch the key online (default true) |
| `SCIRPOPHAGA_URL` | Online key source (used when `USE_ONLINE_KEY=true`) |
| `BIN_C2` | Offline key, hex (used when `USE_ONLINE_KEY=false`) |

### Mock-only

| Variable | Description |
|----------|-------------|
| `FAIL_CHANCE` | Mock random failure probability (default 0.3) |

---

## Cluster Management

Use [cuscutactl](cuscutactl/README.md) for day-to-day operations.

> [!TIP]
>
> The `accounts.txt` format is `account_email:password`, one entry per line.

```shell
# Health check (Kubernetes mode)
# cuscutactl --mode kubernetes --kube-namespace cuscuta doctor

# Health check (Legacy mode)
cuscutactl --mode legacy --postgresql-url "..." --redis-url "..." doctor

# View account overview
cuscutactl --mode legacy --postgresql-url "..." accounts status --max-count 20

# Batch-import accounts
cat accounts.txt | cuscutactl --mode legacy --postgresql-url "..." accounts row add --stdin

# Inspect job results
cuscutactl --mode legacy --redis-url "..." jobs result --code 123456789 --print-detail
```
