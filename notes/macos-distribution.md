# macOS 版の配布と Gatekeeper

Developer ID 署名と公証なし (ad-hoc 署名のみ) の rkpack.app を配ったときの挙動の記録。
時系列で追記する。結論が覆った場合も古い項目は消さず、「訂正」として追記する。

## 2026-11-26 確認 1: CI の artifact をブラウザ経由で起動

### 方法

- release ワークフローの手動実行 (run 45722456208, macos-build ブランチ) で作った `rkpack-macos` artifact を、Mac の Chrome でダウンロードした。
- Finder で展開した (artifact の zip の中に zip があるので 2 回)。

### 観察 (事実)

- `xattr -l ~/Downloads/rkpack.app` の結果:
  - `com.apple.macl:`
  - `com.apple.quarantine: 0081;6b0792cc;Chrome;4EA33882-C178-4889-ABB6-A54ECEBD23B6`
- ダブルクリックすると、次のダイアログが出て起動しなかった。ボタンは「Finder に表示」と「OK」。
  - 「"rkpack.app" が悪質なソフトウェアかどうかを Apple では確認できないため、このソフトウェアは開けません。」
  - 「このソフトウェアはアップデートが必要です。詳しくは開発元にお問い合わせください。」
  - 「このファイルは "Chrome" により今日の 11:00 にダウンロードされました。」
- 「システム設定 > プライバシーとセキュリティ」の「セキュリティ」欄に「"rkpack.app" は開発元を確認できないため、使用がブロックされました。」と「このまま開く」ボタンが出た。
- 「このまま開く」を押したあと、rkpack.app は起動した。途中で出たダイアログの内容は記録していない。

### 仮説

- quarantine は、Chrome が zip に付けたものが Finder (アーカイブユーティリティ) の展開で中身に引き継がれたものと思われる。
- 「アップデートが必要です」の一文が出る条件は分かっていない。ad-hoc 署名か bundle の構成が関係している可能性がある。

### 未確認

- `xattr -dr com.apple.quarantine` で外したときに、警告なしで起動するか。
- 同梱した `.command` や `.txt` が quarantine 付きでどう扱われるか。
- 起動後の GUI の表示、master.db の読み込み、pack/unpack。
