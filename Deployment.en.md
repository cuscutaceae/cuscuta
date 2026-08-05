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
# Edit my-values.yaml: postgresql.url, redis.url, chilo.constants, github.*, api.*

# 2. Install
helm install cuscuta oci://ghcr.io/cuscutaceae/charts/cuscuta --version 0.1.0 -f my-values.yaml

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
# Edit docker-compose.override.yaml: GITHUB_*_TOKEN and chilo constants
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
| `chilo.constants.binC1` | Challenge constant C1 (hex) |
| `chilo.constants.binC2` | Challenge constant C2 (hex) |
| `chilo.constants.binLoginC31` | Challenge constant login-C31 (hex) |
| `chilo.constants.binLoginC32` | Challenge constant login-C32 (hex) |
| `github.bundleRepository` | GitHub repo for bundle data |
| `github.bundlePath` | Path to bundle JSON in repo |
| `github.bundleToken` | GitHub PAT for bundle repo |
| `github.songRepository` | GitHub repo for song list |
| `github.songPath` | Path to song list in repo |
| `github.songToken` | GitHub PAT for song repo |
| `api.login` | Target login API endpoint |
| `api.listFriends` | Target friend list API endpoint |
| `api.addFriends` | Target add-friend API endpoint |
| `api.deleteFriends` | Target delete-friend API endpoint |
| `api.getRank` | Target rank API endpoint |
| `worker.keda.enabled` | Enable KEDA autoscaling |
| `worker.keda.maxReplicaCount` | Maximum worker replicas |
| `mock.enabled` | Enable mock service (disable in production) |

---

## Environment Variables

All cuscuta components receive configuration through environment variables.

### Common variables

| Variable | Used by |
|----------|---------|
| `RUST_LOG` | all |
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

### Worker-only variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WORKER_MAX_JOBS` | 8 | Max concurrent jobs per worker |
| `WORKER_MAX_RETRIES` | 30 | Max API retry attempts |
| `WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS` | 10 | Exponential backoff base (ms) |
| `WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER` | 2 | Exponential backoff multiplier |
| `WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS` | 500 | Max backoff delay (ms) |
| `WORKER_ACCOUNT_LEASE_TIME_SECS` | 120 | Account lease duration (s) |
| `WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS` | 30 | Lease refresh interval (s) |
| `WORKER_JOB_MAX_WORK_TIME_SECS` | 1200 | Maximum working time for a job (s) |
| `WORKER_EMPTY_FRIENDS_DELAY_TIME_SECS` | 10 | Rate limiting delay time (s) |
| `ETA_ENABLE` | true | Enable ETA estimation |
| `API_CHILO` | — | chilo service URL |
| `API_LOGIN` | — | Login API endpoint |
| `API_LIST_FRIENDS` | — | Friend list API endpoint |
| `API_ADD_FRIENDS` | — | Add friend API endpoint |
| `API_DELETE_FRIENDS` | — | Delete friend API endpoint |
| `API_GET_RANK` | — | Rank API endpoint |

### Chilo-only variables

| Variable | Description |
|----------|-------------|
| `BIN_C1` | Constant C1 (hex, 32 bytes) |
| `BIN_C2` | Constant C2 (hex, 32 bytes) |
| `BIN_LOGIN_C31` | Constant login-C31 (hex, 10 bytes) |
| `BIN_LOGIN_C32` | Constant login-C32 (hex, 10 bytes) |

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
