# Contributing

Thanks for helping out! 

1. Fork and create a feature branch.
2. Windows uses the WPD API (`src/device/wpd.rs`); Linux uses GVFS mounts (`src/device/gvfs.rs`). Both implement the traits in `src/device/traits.rs`. You can test the Linux backend anywhere by pointing `PHOTO_EXTRACTION_GVFS_DIR` at a fake mount directory, and type-check the Windows backend with `cargo check --target x86_64-pc-windows-msvc`.
3. Run `cargo fmt` and `cargo test` before opening a PR.
4. Mock devices (`photo_extraction_tool test run-quick`) let you test extraction logic without a phone.
5. Open the PR describing what changed and why. Bug reports: include device model, OS version and logs.
