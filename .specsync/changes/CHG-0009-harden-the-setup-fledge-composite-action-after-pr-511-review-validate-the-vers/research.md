---
change: CHG-0009-harden-the-setup-fledge-composite-action-after-pr-511-review-validate-the-vers
artifact: research
---

# Research

Everything below was measured against the real `CorvidLabs/fledge` repo and a
real `curl`, not inferred from documentation.

## 1. Sidecar coverage across every release

`gh release list --limit 100` → 39 releases, each queried with
`gh release view <tag> --json assets`:

| Releases | Binaries | Sidecars |
|----------|----------|----------|
| v1.4.0 – v1.7.2 | 5 | 5 |
| v0.9.1 – v1.3.1 | 4 | 4 |
| v0.6.0 – v0.9.0 | 0 | 0 |
| v0.3.0 – v0.5.0 | 4 | 0 |

Conclusion: making verification mandatory refuses exactly v0.3.0, v0.4.0 and
v0.5.0. v0.6.0–v0.9.0 publish no assets at all, so the action could never have
installed them regardless. This is what makes removing the skip path cheap.
v0.5.0 doubles as the CI fixture for the missing-sidecar case: it really does
publish `fledge-linux-x86_64` with nothing beside it.

## 2. The traversal is real, not theoretical

```
$ curl -sv "https://github.com/CorvidLabs/fledge/releases/download/../../../octocat/Hello-World/x"
> GET /CorvidLabs/octocat/Hello-World/x HTTP/2
```

curl resolves the dot segments before sending. The request that leaves the
runner is for a different repository's path, so `version` reaches further than
the tag position it appears to occupy.

## 3. curl flag semantics the script depends on

- `-f` + `-w '%{http_code}'`: curl still writes the status code to stdout when
  `--fail` aborts the transfer. A 404 gives exit 56 (or 22) with `404` on
  stdout; a DNS failure gives exit 6 with `000`. Both signals are therefore
  available at once, which is what lets the error distinguish "no sidecar
  published" from "the request never completed".
- `--retry 3` alone does **not** retry a 404: the sidecar 404 returns in 0.35s.
  Adding `--retry-all-errors` makes the same 404 retry four times through the
  backoff. Hence the deliberate asymmetry between the two fetches.
- `--config -` reads options from stdin, so `header = "Authorization: Bearer …"`
  never appears in argv. Confirmed effective by the bad-token case returning
  HTTP 401 — an anonymous request would have returned 200.

## 4. Windows

`gh release view v1.7.2 --json assets` lists `fledge-windows-x86_64.exe` and
`fledge-windows-x86_64.exe.sha256`. The reviewer's factual correction is
right: the binary ships, and CHG-0008's error text implied otherwise.

## 5. `jq` availability

`jq` is preinstalled on `ubuntu-latest`, `macos-latest` and `windows-latest`
GitHub-hosted images, so using it inside the action adds no install step. The
dependency-footprint argument that justified `grep | cut` in CHG-0008 applies
to `install.sh` (which runs on arbitrary user machines), not here.
