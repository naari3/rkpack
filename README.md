# rkpack

rekordbox の master.db からプレイリストと音声ファイルを `.rkp` にパック / アンパックするツール。
エクスプローラーからダブルクリックすると GUI、ターミナルから引数付きで実行すると CLI として動く。

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

## リリース

`Cargo.toml` の `version` を上げてから、同じ番号の `v` 付きタグを push する。

```
git tag v0.1.0
git push origin v0.1.0
```

`.github/workflows/release.yml` が Windows で `scripts/package.ps1` を実行し、zip を添付した GitHub Release を作る。
タグと `Cargo.toml` の version が一致しない場合はワークフローが失敗する。
`workflow_dispatch` で手動実行した場合は Release を作らず、workflow artifact として zip を残すだけ。
