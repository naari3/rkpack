# rkpack

rekordbox の master.db からプレイリストと音声ファイルを `.rkp` にパック / アンパックするツール。
エクスプローラー / Finder からダブルクリックすると GUI、ターミナルから引数付きで実行すると CLI として動く。

## ビルド

```
cargo build --release
```

## 配布用 zip の作成

```
pwsh scripts/package.ps1
```

`dist/rkpack-v<version>-<host triple>.zip` (中身は `rkpack.exe` のみ) が生成される。
`-SkipBuild` を付けるとビルドを飛ばして既存の `target/release/rkpack.exe` を固める。

### macOS

```
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo install cargo-bundle
scripts/package-macos.sh
```

`dist/rkpack-v<version>-universal-apple-darwin.zip` (中身は universal バイナリの `rkpack.app`) が生成される。
Developer ID 署名と公証はしておらず、ad-hoc 署名のみ。

## macOS での初回起動

ダウンロードした `rkpack.app` は Gatekeeper に止められる。次のどちらかで許可する。

- 一度開こうとしてから「システム設定 > プライバシーとセキュリティ」の「このまま開く」を押す
- ターミナルで quarantine 属性を外す

```
xattr -dr com.apple.quarantine /Applications/rkpack.app
```

## リリース

`Cargo.toml` の `version` を上げてから、同じ番号の `v` 付きタグを push する。

```
git tag v0.1.0
git push origin v0.1.0
```

`.github/workflows/release.yml` が Windows で `scripts/package.ps1`、macOS で `scripts/package-macos.sh` を実行し、両方の zip を添付した GitHub Release を作る。
タグと `Cargo.toml` の version が一致しない場合はワークフローが失敗する。
`workflow_dispatch` で手動実行した場合は Release を作らず、workflow artifact として zip を残すだけ。
