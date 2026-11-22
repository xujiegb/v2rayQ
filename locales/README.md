# Language resources

`client/` supplies egui, Web, LuCI, Cockpit and CLI clients. `server/` supplies the standalone Web server and Cockpit server. Applications share localization mechanics and keep their text and language preferences independent.

Each UTF-8 JSON file is named with its language code, for example `fr.json`. `_name` is the native name shown in the language selector. Other entries map stable English UI phrases to translated text. Placeholders such as `{count}` and `{name}` must appear with the same spelling and count in every translation. User-entered names, endpoints, protocol identifiers, JSON property names and machine RPC replies retain their original values.

To add a language:

1. Copy the relevant application's `en.json` to a new language-code filename.
2. Set `_name` and translate its values. Add a separate file in the other application if that application also supports the language.
3. Run `python3 tools/check_locales.py`, then rebuild the Rust applications or reinstall the plugin assets.

`embed.rs` discovers the files and embeds the catalogs and selector manifest at build time. `tools/prepare_locales.py client|server DESTINATION` packages the same files and generates `index.json` for Cockpit and LuCI. No generated catalog copies are checked into individual frontends. Missing translations fall back to English at runtime; CI requires complete catalogs and matching placeholders. English catalogs are required in both folders.

# Portable client configuration

Settings exports UTF-8 JSON with this envelope:

```json
{
  "format": "v2rayq-client",
  "version": 1,
  "profile": {
    "nodes": [],
    "subscriptions": [],
    "selected_node": null,
    "settings": {}
  }
}
```

`profile` uses the shared client model. The document exchanges saved profile data among clients; a future Core adapter consumes a separate schema. The server keeps its drafts in its own store.

Export includes node credentials and subscription URLs. It omits transient subscription refresh results. Import accepts up to 4 MiB, rejects unsupported versions and invalid references, and validates record limits before persisting a single storage transaction. Preview performs the same validation and merge planning without saving a profile.

Merge rules:

- Subscriptions match by normalized HTTPS URL. Existing names and enabled states are preserved for matches.
- Nodes match by protocol configuration, endpoint and mapped subscription ownership. Existing names win for duplicates. New nodes and subscriptions receive fresh IDs, preserving their relationships.
- Existing selection is preserved. An empty destination selects the mapped imported selection, or its first node.
- Route rules match by rule content. New rules receive fresh IDs and are appended after existing rules.
- Current scalar preferences stay in place by default. Selecting **Use imported preferences** also adopts the imported language, DNS, routing mode and requested network flags. Actual service status is queried separately; merge never starts services or retrieves subscriptions.

The HTTP and RPC command envelope is bounded at 8 MiB to allow JSON escaping around a portable configuration. CLI export writes the same document to stdout; merge reads it from stdin.
