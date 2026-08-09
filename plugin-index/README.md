# Slipstream plugin catalog

`v1/index.json` is the built-in catalog fetched by the host. The catalog is intentionally empty
until reviewed plugin packages are publicly available.

The host verifies `v1/index.json.sig` with the Ed25519 key pinned in
`crates/slipstream-host/src/ops/store/sources.rs`. Sign the exact index bytes with the offline key:

```sh
openssl pkeyutl -sign -rawin \
  -inkey /secure/path/plugin-index-ed25519.pem \
  -in plugin-index/v1/index.json \
  | base64 -w0 > plugin-index/v1/index.json.sig
printf '\n' >> plugin-index/v1/index.json.sig
```

Never commit the private key. Add an entry only after its exact package version and integrity hash
have been reviewed.
