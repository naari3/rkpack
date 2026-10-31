# rekordbox の ID 採番方式の調査

rekordbox が master.db の各テーブルの ID をどう採番しているか、rkpack の採番 (`get_max_numeric_id` + 1) と比べて問題がないかを調べた記録。
時系列で追記する。結論が覆った場合も古い項目は消さず、「訂正」として追記する。

## 2026-10-08 調査 1: 手元 master.db の実データと pyrekordbox

### 方法

- `%APPDATA%/Pioneer/rekordbox/master.db` をコピーし (WAL は空)、コピーを `rkpack export` で平文化して sqlite3 で集計した。
- rekordbox が入れた行と rkpack が入れた行は、`rb_local_usn` の有無で分けた。rkpack は `rb_local_usn` を設定しないので、rkpack が入れた行は NULL になる。ただし `rb_local_usn` を rekordbox 自身が設定しないテーブルもあり、その場合は ID の形式と参照先で判断した。
- rekordbox のバージョンは 7.2.3 (agentRegistry の LangPath より)。DB の作成は 2021-06 (rekordbox 6 時代)。
- pyrekordbox (dylanljones/pyrekordbox, 069e46f) の `pyrekordbox/masterdb/database.py` を読んだ。

### 観察 (事実)

#### djmdContent.ID

- 全 7490 行。そのうち rekordbox が入れた行 (`rb_local_usn` あり) は 7452 行で、min 88741、max 268428150 (< 2^28 = 268435456)。
- ID の bit 長の分布は 28bit が 3783、27bit が 1857、26bit が 932 ... で、1bit 下がるごとにほぼ半分になる。`[0, 2^28)` の一様乱数と整合する。
- created_at 順に並べても ID は単調増加しない (連続する 2 行の比較で、増加は 3798 / 7489 でほぼ半々)。
- 全行で `MasterSongID == ID`。
- `rb_file_id` は ID とは別の値で、2^53 前後の大きな数値 (例 15762598696803311)。
- `UUID` 列は UUID v4 形式の文字列。

#### マスタ系テーブル (rekordbox が入れた行)

- djmdArtist, djmdAlbum, djmdGenre, djmdKey, djmdLabel, djmdPlaylist, djmdMyTag, djmdHistory, djmdCloudFilterPlaylist などは、ID が `[0, 2^32)` に散らばっている (32bit が約半分、31bit が約 1/4 ...)。
- djmdCue も同様に 32bit 範囲に散らばっている (`rb_local_usn` は全行 NULL)。
- djmdColor, djmdCategory, djmdMenuItems, djmdSort は 1 からの小さい連番 (固定のマスタ)。

#### UUID を ID に使うテーブル

- contentFile, djmdSongPlaylist, djmdMixerParam, contentCue, djmdSongHistory は、rekordbox が入れた行の ID が UUID 文字列。
  - contentFile は `<UUID>_<URL エンコードしたパス>` 形式 (例 `4fa724e7-..._%2FPIONEER%2FUSBANLZ%2F...ANLZ0000.DAT`)。
- これらのテーブルにある数値 ID の行は、すべて rkpack が入れたもの。
  - 例: djmdSongPlaylist の数値 ID は 99082039..99082113。UUID の行に `99082038-9192-4bc1-...` があり、`MAX(CAST(ID AS INTEGER))` は SQLite の仕様で先頭の数字部分だけを読むので 99082038 になる。その +1 から連番になっている。contentFile (99110241 → 99110267..)、djmdMixerParam、contentCue も同じ。

#### rkpack が入れた行

- djmdContent: 38 行 (268428151..268428188 の連番)。パスは `F:/rkpack/...` など。
- djmdCue: 4294330000 以上の 268 行が連番で、そのうち 267 行が rkpack で入れた djmdContent を参照している。
- djmdArtist / Album / Genre / Key / Playlist も、32bit 上限近くに rkpack 由来の連番がある。

#### pyrekordbox の実装

- `generate_unused_id(table, is_28_bit=True)`: `secrets.token_bytes(4)` から 32bit 値を作り、`is_28_bit` なら 4bit 右シフトする。100 未満は捨て、未使用になるまで引き直す。
- `add_content` は ID と `rb_file_id` を `generate_unused_id` で作り、`MasterSongID = ID`、`UUID = uuid4()` を設定する。
- アルバム、プレイリストなども `generate_unused_id` (デフォルトの `is_28_bit=True`) を使う。実 DB ではマスタ系の ID は 32bit 範囲なので、pyrekordbox のこの部分は実データと一致しない。
- `add_to_playlist` は djmdSongPlaylist の ID と UUID 列に、それぞれ別の `uuid4()` を入れる。実 DB の UUID 形式の ID と一致する。

### 仮説 (推論)

- rekordbox は djmdContent.ID を 28bit の一様乱数で採番し、未使用であることを確認していると思われる。分布と単調性のなさは、乱数による採番と整合する。
- マスタ系と djmdCue は 32bit の一様乱数と思われる。
- contentFile / djmdSongPlaylist / djmdMixerParam / contentCue / djmdSongHistory は UUID v4 と思われる。
- djmdContent だけ 28bit に収めている理由は不明。他の何か (デバイスエクスポート時の track ID、上位 bit を別用途に使う内部表現など) で上限がある可能性がある。未検証。

### rkpack への影響 (観察から言えること)

- rkpack の `get_max_numeric_id` + 1 は、rekordbox の採番方式 (乱数 / UUID) と一致していない。
  - djmdContent: rkpack の採番はすでに 268428188 まで来ている。2^28 まで残りは 7267。これを超えると 29bit の ContentID ができる。rekordbox がそれを受け付けるかは未検証。
  - マスタ系 / djmdCue: 2^32 上限 (4294967295) に近い所から連番で採番している。djmdArtist は残り約 31 万。
  - UUID テーブル: rekordbox は UUID 文字列を使っているが、rkpack は数値を入れている。現状 rekordbox は読み込めているように見えるが、同期や書き出しで問題が出ないかは未検証。
- `rb_local_usn` を NULL のまま入れている。rekordbox は `agentRegistry.localUpdateCount` (現在 174109) をもとに USN を採番していると思われ、pyrekordbox もこれをインクリメントして各行の `rb_local_usn` に入れている。NULL の行が同期 / クラウド / 差分処理でどう扱われるかは未検証。

### 未確認・次の手

- 28bit の理由と、28bit を超える ContentID を rekordbox が受け付けるか。rekordbox.exe の静的解析 (乱数生成と 0x0FFFFFFF のマスク / 4bit シフト箇所を探す) か、テスト用 DB に 2^28 以上の ContentID を入れて rekordbox で読み込み、USB エクスポートしてみる。
- rkpack の採番を rekordbox に合わせるか (djmdContent は 28bit 乱数、マスタ系 / Cue は 32bit 乱数、UUID テーブルは uuid v4、`rb_local_usn` の採番)。

## 2026-10-08 調査 2: 決定的な規則性があるか

きっかけ: ユーザーから「他人の環境からもらった曲でも、自分の環境で ContentID が合っているように見えた」との指摘。乱数ではなく、ファイルやパスから決まる規則がある可能性を確かめた。
(どの画面で何と何が一致して見えたのかは未確認)

### 観察 (事実)

- **同じファイルの二重登録**: rekordbox 同梱のサンプラー音源 (`MergeFX Sample Sound 202.wav` など) が、2021-06 に `Music/PioneerDJ/Sampler/...`、2026-03 に `Music/rekordbox/Sampler/...` として別々に登録されている。contentFile.Hash は同一で、ContentID は別 (202: 71590864 と 136864978、203: 187862521 と 133646718 など)。
- **他人の環境の .rkp** (`Downloads/95 SugeeOTAKU.rkp`、パック元の MasterDBID 3490033746 / DeviceID 2151321c-...、自分の DB は 740897526 / 8b4f9dba-...):
  - パック内の 39 曲の ContentID は、自分の DB に 1 つも存在しない。
  - 自分も rekordbox で独自に登録している曲は ID が違う。例: Sonozaki はパック 109190599 / 自分 103679821、Not Her Karma はパック 231280062 / 自分 41350250、Get Deaded (Euph Remix) はパック 237348268 / 自分 171741896。
  - パック内の曲も `MasterSongID == ID` で、28bit 以下 (25〜28bit)。
- **パス・UUID のハッシュとの照合**: rekordbox が入れた 400 行で、FolderPath (そのまま / バックスラッシュ / 小文字)、FileNameL、UUID (ハイフンあり / なし)、rb_file_id を UTF-8 / UTF-16LE にし、CRC32 / Adler32 / FNV-1a / MD5 / SHA1 / SHA256 (先頭 4 バイト BE/LE、末尾 4 バイト BE) を取って、`>>4` と `&0x0FFFFFFF` の両方と比べた。一致は 0 件。
- ID の下位 4bit と、28bit 中の上位 4bit の分布はどちらもほぼ均等 (各 410〜510 件 / 7452 件)。
- `rb_file_id` は ID とは別の構造に見える (下位 32bit が 1006573〜1006589 と近い値に固まっている)。未調査。

### 仮説 (推論)

- ContentID はファイル内容、ファイル名、パスのいずれかから決定的に計算されているわけではない可能性が高い。同じ内容で別パスなら別 ID、同じ曲でも別環境なら別 ID で、試したハッシュ関数とも一致しない。乱数による採番 (調査 1 の仮説) と矛盾しない。
- 「合っているように見えた」原因の候補 (未確認):
  - 以前のプレビューの ContentID 欄は、パック内の ID (他人の環境の ID) をそのまま表示していた。実行時には使われない値だったので、他人の ID が自分の環境でもそのまま入るように見えた可能性がある。
  - 重複列の「Hash一致」は ID の一致ではなく、contentFile.Hash の一致による紐付け。
  - 試していない関数 (別のハッシュや、パス以外の入力) で決定的に作られている可能性は、このデータだけでは排除しきれない。

### 訂正: contentFile.Hash は音声ファイルのハッシュではない

- 観察: contentFile の Path の拡張子は 2EX / DAT / EXT が各 7480、JPG 5552、LYRIC 261、3EX 60。音声ファイルの行はない。
- 先にユーザーへ「パック内の音声ファイルのハッシュ (contentFile.Hash) で重複判定している」と説明したのは誤り。正しくは、解析ファイル (ANLZ) とアートワークのハッシュ。GUI の一致理由の文言も同じ誤りだったので修正した。

### rkpack の重複検出の問題 (観察)

- artwork.jpg のハッシュは、同じアルバムアートを持つ別の曲同士で一致する。例: パック曲 `Into The Abyss` / `Humanoid interface for contact` / `The 2400m that defines the inv...` の 3 曲が、それぞれ artwork.jpg で同じ 3 つの ContentID (268428163 / 268428166 / 268428173) にヒットする。パック曲 207106796 の artwork.jpg は 11 曲にヒットする。
- `detect_duplicate_contents` と `load_unpack_preview` は contentFile の行ごとに `LIMIT 1` で 1 件を拾う。`load_unpack_preview` はループ中で後勝ちになる。ANLZ の行が後にあり、それも一致すれば正しい曲に落ちる。一方、アートワークだけが一致して ANLZ が一致しない場合は、別の曲を「重複」と判定してしまう。この状態で「更新する」を選ぶと、別の曲のキューが削除される。
- ANLZ が一致するのは、解析ファイルがバイト単位で同一の場合だけ (rkpack で同じパックを再度取り込んだ場合など)。同じ音源でも自分の rekordbox で解析し直したものは、ANLZ のハッシュが一致しない可能性が高い (未検証)。

### 訂正: 独立に解析した曲でも ANLZ のハッシュは一致しうる

- 直前の「自分の rekordbox で解析し直したものは ANLZ のハッシュが一致しない可能性が高い」は誤り。
- 観察: パック曲と、自分の DB に元から rekordbox で登録されている曲の比較。
  - Not Her Karma (41350250): DAT / EXT / 2EX の 3 つとも一致。
  - Sonozaki (103679821)、aim at our school (22310012): 2EX だけ一致。
  - Get Deaded (Euph Remix) (171741896): artwork.jpg だけ一致し、ANLZ はどれも一致しない。
- 仮説: 解析結果はバージョンや設定が同じなら同一になりやすく、2EX は特に一致しやすいと思われる。いずれにせよ、ANLZ / artwork の一致は「同じ曲」の決定的な証拠にはならない (artwork は別の曲でも一致し、ANLZ は同じ曲でも一致しないことがある)。

## 2026-10-08 調査 3: 実装前の確認

### 訂正: rb_local_usn は djmdCue 以外のほぼ全テーブルに付いている

- 調査 1 の「rb_local_usn を rekordbox 自身が設定しないテーブルもあり」「djmdSongPlaylist などは全行 NULL」は、集計スクリプトが数値 ID の行しか数えていなかったことによる誤り。
- 観察 (全行を数え直した結果): rekordbox が入れた行には、contentFile 28171/28313、contentCue 364/402、djmdMixerParam 7452/7490、djmdSongPlaylist 2243/2318、djmdSongHistory 1404/1404 に rb_local_usn が付いている。NULL の行はすべて rkpack が入れた数値 ID の行。
- djmdCue だけは全 2566 行が NULL。
- 全テーブルの rb_local_usn の最大値 174109 は `agentRegistry.localUpdateCount` (int_1) と一致する。djmdContent では usn の重複がない (NULL を除く)。
- pyrekordbox のテスト DB (rekordbox 6) でも、contentFile と djmdMixerParam の行に rb_local_usn がある。
- pyrekordbox の `autoincrement_local_update_count`: 変更 1 件ごとに localUpdateCount を +1 し、その値を行の rb_local_usn に入れる。最後に localUpdateCount を更新する。

### contentFile.ID の形式

- 観察: rekordbox が入れた 28171 行のすべてで、`ID == <djmdContent.UUID>_<Path を英数字と "_.-~" 以外すべて %XX にしたもの>` が成り立つ (Python の `urllib.parse.quote(path, safe='')` と同じ結果)。サンプルに非 ASCII のパスはなかった。
- contentFile.UUID 列は ID とは別の UUID v4。

### 空テーブルの扱い (データなし)

- djmdActiveCensor, contentActiveCensor, djmdSongMyTag, djmdSongTagList, djmdSongHotCueBanklist, hotCueBanklistCue, djmdHotCueBanklist は、手元の DB にも pyrekordbox のテスト DB にも行がない。ID 形式は不明。
- 実装上の仮置き (推論): djmdActiveCensor は djmdCue と同じ「詳細行」なので 32bit 乱数とする。contentActiveCensor は contentCue と、djmdSong* は djmdSongPlaylist / djmdSongHistory と同系統とみなして UUID にする。hotCueBanklistCue は contentCue と同系統とみなして UUID にする。djmdHotCueBanklist はマスタ系として 32bit 乱数にする。Ghidra の調査で分かれば直す。

## 2026-10-08 実装: rekordbox の採番方式に合わせる

### 変更内容

- `src/core/id_mapping.rs`
  - `IdKind` / `id_kind_for_table` / `IdAllocator` を追加し、`get_max_numeric_id` (最大値 + 1) を廃止した。
  - djmdContent は [100, 2^28) の乱数、contentFile は `<djmdContent.UUID>_<パーセントエンコードした Path>`、UUID 系テーブル (調査 3 の表) は uuid v4、それ以外は [100, 2^32) の乱数。
  - DB 上の既存行 (論理削除済みを含む) と、セッション内で払い出し済みの ID を避ける。下限の 100 は pyrekordbox に合わせた。djmdColor など 1 からの固定マスタとの衝突を避ける意味もある。
  - `UsnCounter` を追加した。挿入する行ごとに `localUpdateCount` を +1 して rb_local_usn に入れ、コミット前に `localUpdateCount` を書き戻す。djmdCue には付けない。
- `src/core/unpack.rs`
  - マスタ / ContentID / 関連テーブル / プレイリストの採番を `IdAllocator` 経由にした。
  - プレビューで新規トラックに割り当てる ContentID も 28bit 乱数にした。
  - ユーザー指定の ContentID は 2^28 未満でなければエラーにする。
  - contentFile の ID が既存と衝突した場合 (同じパックを新規として二重に取り込んだ場合など) は、警告を出して先頭の UUID を新しく作ったもので代用する。
  - `generate_content_ids` を公開し、GUI の「新規として追加」で使うようにした。

### 検証 (観察)

- 手元 DB の平文コピーで contentFile.Hash を空にし (全曲を新規扱いにするため)、`APPDATA` を scratchpad に向けて CLI の unpack で `95 SugeeOTAKU.rkp` (39 曲) を取り込んだ。
  - 挿入 562 行。djmdContent 39 行はすべて 28bit 以下の数値。djmdPlaylist は 32bit 数値。djmdSongPlaylist / djmdMixerParam / contentCue は uuid v4。contentFile 134 行はすべて rekordbox 形式の式と一致した。
  - djmdCue 以外の 291 行に rb_local_usn 174110..174400 (重複なし) が付き、localUpdateCount は 174109 → 174400 になった。djmdCue 271 行は NULL。
  - 新規行の ContentID / PlaylistID の参照切れ、MasterSongID != ID、contentCue の JSON 内の Cue ID の参照切れは 0 件。
- 未検証:
  - 取り込んだ DB を rekordbox で開いて、表示や USB 書き出しができるか。
  - GUI 経路 (`unpack_playlist_with_decisions`、ContentID 指定あり)。CLI と同じ関数を通るが、実際には動かしていない。
  - contentFile の ID が衝突したときの代用経路。
  - マスタ系の新規採番。今回のパックはアーティストなどがすべて既存と名前一致したため、新規行が出なかった。

### 残っている問題 (今回は手を付けていない)

- 重複検出が contentFile.Hash (ANLZ と artwork) の一致に頼っていて、別の曲を誤検出しうる (調査 2)。
- djmdContent.UUID や各行の UUID 列は、パックに入っていた値をそのまま使っている。同じパックを新規として二重に取り込むと UUID が重複し、解析ファイルのパスも共有される。
- 「更新する」で既存曲の関連行を物理削除している。rekordbox は rb_local_deleted による論理削除を使っているように見えるが、未確認。

## 2026-10-08 調査 4 と実装: 重複検出の見直し

### ユーザーの観察と、その説明

- ユーザーの観察: 他人の環境で一から作った .rkp (`95 SugeeOTAKU.rkp`、パック元の環境では rkpack での取り込みはしていない) を 13:40 に取り込んだとき、9 曲が「重複」でスキップされ、その対応は正しそうだった。
- 説明 (観察に基づく): このスキップは ID の一致ではなく、contentFile.Hash の一致による重複検出によるもの。該当したのは、自分の DB に元から rekordbox で登録されていた 9 曲 (Sonozaki 103679821、choke them, baby 261751554、aim at our school 22310012、naari3 の曲 232592112、ID 207106796 の曲 → 210614121、M2-9 219481260、Not Her Karma 41350250、Get Deaded 171741896、Dynamite 180099195)。ContentID はどれもパック内の ID と違う。
- 仮説: 同じ音源を rekordbox で解析すると、ANLZ (特に 2EX) がバイト単位で同じになりやすい。また曲に埋め込まれたアートワークから作る artwork.jpg も同じになる。そのため、別の環境で解析した同じ曲でも Hash が一致した。

### 観察 (パック 39 曲 × 現在の DB のコピー、scratchpad の dupdesign.py)

- 候補は「contentFile.Hash の一致 (ANLZ か artwork か)」と「djmdContent.FileSize の一致」から集め、各候補の FileSize / Length の一致と、パック内の音声と既存ファイルのバイト比較を見た。
- artwork だけ一致した候補 (誤検出の原因) は、すべて FileSize も Length も違い、バイト比較も不一致だった。例: Into The Abyss に対する 268428166 / 268428173、ID 207106796 の曲に対する 10 件。
- 以前の 9 件の内訳:
  - バイト一致: 2 件 (Get Deaded は artwork だけの一致だったが、音声はバイト一致。ID 207106796 の曲 → 210614121)。
  - バイト不一致で FileSize も違うが、ANLZ (2EX は全件、DAT / EXT は一部) が一致し、Length も同じ: 7 件。タグなどが違う同じ音源の別ファイルと思われる。
- Hash ではどれも一致しないのに FileSize で見つかり、バイト一致した例があった (241031 Sakurako Dub → 216245423、`Downloads/241031 Sakurako Dub.mp3`)。Hash だけの検出では見逃していた。

### 実装 (`find_duplicate`)

- 候補: contentFile.Hash の一致 (ANLZ か artwork かを区別) と FileSize の一致。
- 判定:
  - 音声一致 (`AudioIdentical`): FileSize が同じで、パック内の音声と既存の FolderPath のファイルがバイト単位で一致。
  - 解析一致 (`SameAnalysis`): ANLZ のいずれかの Hash が一致し、Length も同じ。
  - artwork だけの一致は重複とみなさない。
- 複数ある場合は、音声一致 > 解析一致の順に選ぶ。同じ順位なら ANLZ 一致あり > created_at が古い順。件数は `candidate_count` として GUI に出す。
- CLI の `detect_duplicate_contents` も同じ判定に揃えた。以前は contentFile の行ごとに確認を出していたので、1 曲について複数回確認が出ることがあった。今は曲ごとに 1 回。

### 検証 (観察)

- 現在の DB のコピー: 39 曲すべてを検出した。音声一致 32 / 解析一致 7。Into The Abyss / Humanoid interface... / The 2400m... はそれぞれ別の正しい曲 (268428163 / 268428166 / 268428173) を指す。
- rkpack で入れた行 (rb_local_usn NULL の djmdContent 38 件と関連行) を消したコピー (初回取り込み前の再現): 10 件を検出した。以前の 9 件と同じ曲を同じ既存 ID に紐付け、加えて Sakurako Dub (216245423、音声一致) を検出した。
- 未検証: GUI のプレビュー表示。既存ファイルが見つからない (移動 / 削除された) 場合は、解析一致でしか拾えない。
- 判断の余地: 現在の DB では Sakurako Dub が 268428169 (rkpack で取り込んだ方、ANLZ 一致あり) に紐付く。216245423 (元から登録されていた方) を優先すべきかは決めていない。

## 2026-10-08 Ghidra 調査の結果の反映

詳細は `notes/id-allocation-ghidra.md` (rekordbox.exe 7.2.3 の静的解析。別エージェントが作成)。ここには rkpack に関係する点だけを書く。

### Ghidra 側の結論 (要約。あちらのノートでは観察と仮説を分けて書いている)

- djmdContent.ID は JUCE の `Random::getSystemRandom().nextInt()` (48bit LCG。seed は起動時の時刻、QPC、ポインタ) を使った棄却法で、[1, 2^28) の値を作る。0、0xFFFFFFFF、上位 4bit が立っている値を引き直す。事前の存在チェックはなく、INSERT が SQLITE_CONSTRAINT で失敗したら作り直す。パスやハッシュから決まる値ではない。調査 2 の仮説と整合する。
- 28bit にする理由は分からなかった。
- マスタ系 / djmdCue / djmdActiveCensor / djmdHotCueBanklist / djmdPlaylist は 32bit 乱数 (0 と 0xFFFFFFFF を除く)。下限があるのは djmdMyTag (5 以上)、djmdSampler (4 以上)、djmdHistory (3000 以上)。
- djmdSongPlaylist / djmdSongMyTag / djmdSongHistory / djmdMixerParam の ID は juce::Uuid の文字列。contentFile / contentCue の ID を作る処理は exe 内に見つからず、rekordboxAgent 側と思われる。
- rb_local_usn は localUpdateCount を +1 して書き戻し、その値を使う。基本は 1 行ごと。djmdSongPlaylist のまとめ追加だけは、1 回の操作で 1 つの USN を全行に入れる。
- DAO の型が `NoLibrarySyncSql` のテーブル (ActiveCensor, Cue, Property, SongHotCueBanklist, SongTagList, Trash 系など) には USN を付けないと思われる。

### 実 DB との照合 (観察)

- djmdSongPlaylist の rb_local_usn は、2243 行に対して種類が 778。1 つの値を 1 つのプレイリストの行が共有している (例: usn 152022 は 90 行、PlaylistID は 1 種類)。まとめ追加で 1 つの USN という Ghidra の結果と整合する。
- djmdMyTag の ID 1〜4 は Genre / Components / Situation / Untitled Column (既定のカテゴリ)。下限 5 と整合する。

### 実装の変更 (調査 3 の仮置きからの逸脱)

- `UsnCounter`: USN を付けないテーブルを djmdCue だけから、djmdCue / djmdActiveCensor / djmdSongHotCueBanklist / djmdSongTagList に広げた (NoLibrarySyncSql のうち rkpack が書き込むもの)。
- djmdSongPlaylist は、1 回の取り込みの全行に 1 つの USN を付ける (`assign_batch`)。
- 下限 100 はそのまま。rekordbox の下限 (MyTag 5、Sampler 4) より厳しいので予約値とは衝突しない。djmdHistory (3000 以上) は rkpack が書き込まない。djmdContent の 1〜99 を使わない点は rekordbox と違うが、範囲が狭くなるだけで問題はないと判断した。
- 再検証 (全曲新規扱いの E2E): 挿入 562 行。USN は 253 個 (174110..174362、重複なし) で、djmdSongPlaylist の 39 行は 1 つの USN を共有している。参照切れなどの問題は 0 件。
- 未確定のまま残っているもの: djmdSongTagList / djmdSongHotCueBanklist / contentActiveCensor / hotCueBanklistCue の ID 形式 (UUID と仮置き。Ghidra でも分からなかった)。
