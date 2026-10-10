# anirena

Search AniRena and optionally add selected torrents to qBittorrent.

To give selected torrents a ratio limit of 5 and remove their torrent entries
at the share limit while keeping downloaded files:

```sh
cargo run -- --add --qbt-ratio-limit 5 --qbt-remove-at-ratio-limit "search terms"
```

Credentials can be supplied using `ANIRENA_API_KEY`, `QBT_BASE_URL`,
`QBT_USERNAME`, and `QBT_PASSWORD`. The share settings also support
`QBT_RATIO_LIMIT=5` and `QBT_REMOVE_AT_RATIO_LIMIT=true`.

The ratio applies to torrents added by this command. The removal action changes
qBittorrent's **global** share-limit action and also affects existing torrents
when they reach their configured share limits. It keeps downloaded files.
Neither share setting changes unless explicitly supplied. Use only
`--qbt-ratio-limit 5` to retain the server's current share-limit action.
