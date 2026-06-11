# cuscutactl

**This crate is mostly AI=generated**

CLI management tool for [cuscuta](https://github.com/cuscutaceae/cuscuta) clusters.
Talks directly to PostgreSQL and Redis; does not call any cuscuta HTTP API.

## Quick start

```shell
# Health check
cuscutactl --postgresql-url "postgresql://user:pass@localhost:5432/mydb" \
           --redis-url "redis://localhost:6379/0" \
           doctor

# Batch-add accounts from a file (email:password per line)
cat accounts.txt | cuscutactl --postgresql-url "..." accounts row add --stdin

# Inspect job results for a friend code
cuscutactl --redis-url "..." jobs result --code 123456789 --print-detail
```

## Connection modes

| Mode         | Alias   | Description                                        |
|--------------|---------|----------------------------------------------------|
| `legacy`     | `direct`| Direct URLs via `--postgresql-url` / `--redis-url`  |
| `kubernetes` | `k8s`   | Read URLs from Kubernetes secrets via kubectl port-forward |

`legacy` is the default.

## Commands

### `doctor` (alias `check`)

Run diagnostic checks against the databases.

```
cuscutactl doctor
```

Checks performed:

| Check          | Requires         |
|----------------|------------------|
| PostgreSQL     | `--postgresql-url` |
| Migration      | `--postgresql-url` |
| Account stats  | `--postgresql-url` |
| Redis          | `--redis-url`      |
| Job streams    | `--redis-url`      |

Missing URLs are skipped gracefully with a `[ -- ]` marker.

### `jobs` (alias `job`)

#### `jobs status` (alias `stat`)

List all active job streams with message and consumer counts.

```
cuscutactl --redis-url "..." jobs status [--max-count 100]
```

#### `jobs find`

Search result indices for a given friend code.

```
cuscutactl --redis-url "..." jobs find --code <friend_code> [--max-count 100]
```

#### `jobs result` (alias `results`)

Fetch score results for a given friend code.

```
cuscutactl --redis-url "..." jobs result --code <friend_code> \
            [--max-count 100] [--print-detail]
```

Without `--print-detail`, only the total count is shown.

### `accounts` (alias `account`)

#### `accounts status` (alias `stat`)

Show account overview: total, state breakdown, rate<=0, expired leases.

```
cuscutactl --postgresql-url "..." accounts status [--max-count 100]
```

#### `accounts row add`

Add one or more accounts.

```
# Single account
cuscutactl --postgresql-url "..." accounts row add --email user@example.com --password s3cret

# Batch from stdin (email:password per line, blank lines and #-comments skipped)
cat accounts.txt | cuscutactl --postgresql-url "..." accounts row add --stdin
```

#### `accounts row remove`

Delete an account by id.

```
cuscutactl --postgresql-url "..." accounts row remove --id 1
```

#### `accounts row query`

Print full details for a single account. The password is truncated to 4 characters.

```
cuscutactl --postgresql-url "..." accounts row query --id 1
```

#### `accounts rate set`

Adjust or set the rating (`rate`) of an account. By default the value is treated as a delta.

```
# Increment rate by 1 (default: --delta)
cuscutactl --postgresql-url "..." accounts rate --id 1 set --value 1

# Set rate to absolute value 10
cuscutactl --postgresql-url "..." accounts rate --id 1 set --value 10 --delta=false
```

Accounts with `rate <= 0` are never picked up by workers.

#### `accounts rate query`

Print the current rating.

```
cuscutactl --postgresql-url "..." accounts rate --id 1 query
```

#### `accounts release`

Manually set an account back to `Idle`. Refuses to release an account with an active lease unless `--force` is given.

```
cuscutactl --postgresql-url "..." accounts release --id 1
cuscutactl --postgresql-url "..." accounts release --id 1 --force
```

