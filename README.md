# ObsidianCraft

**A note-taking app with a local vault, `[[links]]`, a graph view and Markdown editing — built with [Craft Anything](https://github.com/storytold/craft-anything).**

> **Work in progress.** The rebuild runs and its checks pass one by one; some screens and behaviours
> are still being finished. Treat this as a preview, not a release.

ObsidianCraft keeps your notes as plain Markdown files in a folder you choose (a *vault*), with:

- **Editor**: Markdown editing with live preview, tabs, and the usual shortcuts
- **Links**: `[[wiki links]]`, backlinks and unresolved-link handling
- **Graph view**: a force-directed map of the vault, zoomable and pannable
- **Search and quick switcher**
- **Local vault**: run one or more vaults side by side; no account, no network

Free and open source (MIT). A community app of the [ArtCraft](https://getartcraft.com/) family —
[www.mengbot.com.cn](https://www.mengbot.com.cn).

## Build and run

Requires Rust (edition 2021) and, on Windows, the MSVC toolchain.

```sh
cargo run --release
```

Set `OBSIDIANCRAFT_VAULT` to open a vault folder directly.

## Status

The app builds and draws; the rebuild's checks are being closed one at a time. Everything here is a
study reimplementation of behaviour: no code, assets, icons or text from any commercial program is
included.

## Credits

Built with [Craft Anything](https://github.com/storytold/craft-anything), the clean-room engine that
shows how software works and then builds your own version of it.

This is an independent project. It is not affiliated with, endorsed by, or connected to the authors
of any commercial program it was studied from.

## License

MIT — see [LICENSE](LICENSE).
