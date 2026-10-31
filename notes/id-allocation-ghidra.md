# rekordbox 7.2.3 本体の ID 採番 (静的解析)

rekordbox 7.2.3 (Windows) の `rekordbox.exe` を静的解析し、master.db の各テーブルの ID をどう採番しているかを調べた記録。
時系列で追記する。結論が覆った場合も古い項目は消さず「訂正」を追記する。観察 (アドレス、逆アセンブル / デコンパイル結果、文字列) と仮説 (推論) は分けて書く。

実 DB の観察は `notes/id-allocation.md` (別作業者) を参照。

## 対象と前提

- 対象: `C:\Program Files\rekordbox\rekordbox 7.2.3\rekordbox.exe` (97,779,120 byte, PE32+, image base 0x140000000)
- セクション: .text VA 0x1000 (raw 0x400), .rdata VA 0x318b000 (raw 0x318a400), .data VA 0x58ab000, .pdata VA 0x5c33000
- 以下のアドレスはすべて VA (image base 0x140000000 込み)。
- シンボル (PDB) は無い。関数名は推測でつけたもの。

## 2026-10-08 調査 1: 文字列と capstone による逆アセンブル

Ghidra の自動解析 (headless, 97MB) を待つ間に、Python + capstone で `.text` の RIP 相対 `lea` を全走査して文字列の参照元を引き、`.pdata` で関数境界を取って逆アセンブルした。
(スクリプトは scratchpad に置いた使い捨て。Ghidra での確認は調査 2 以降)

### 観察: 文字列

- `select int_1 as rb_local_usn from agentRegistry where registry_id = 'localUpdateCount'` (0x1438dd9e0)
- `update agentRegistry set int_1 = %d where registry_id = 'localUpdateCount'` (0x1438dd960)
- `insert into __TABLE__ (__COLUMNS__, rb_data_status, rb_local_deleted, rb_local_usn, rb_local_synced, created_at, updated_at) values (__VALUES__, 0, 0, __RB_LOCAL_USN__, 0, CURRENT_TIME, UPDATED_TIME)` (0x1438ddd90)
- `db::AppSyncDBController::insertTrack`, `getAndAddArtistID` など、master.db 操作クラスのメソッド名らしき文字列が多数ある。ただしこれらは `lea` でも絶対ポインタでも参照が見つからなかった (ログ用で未使用の可能性)。
- `db::DevSqliteDBController::*` という別クラスの同名メソッド群もある (デバイス側 DB 用と思われる)。`get_next_id` はこちらにだけある。
- `boost::uuids::detail::random_provider_bcrypt` の文字列もあるが、参照元は 0x1425610b8 からの 1 経路だけで、djmd 系の insert とは別系統。

### 観察: 乱数源 (JUCE の `juce::Random`)

- 0x1427f4b20: `seed = (seed * 0x5DEECE66D + 0xB) & 0xFFFFFFFFFFFF; return (uint32)(seed >> 16);` (`mov rax,[rcx]; imul 0x5deece66d; add 0xb; and 0xffffffffffff; mov [rcx],rax; shr rax,0x10; ret`)
- 0x1427f4b50: スレッドセーフな static 初期化で 0x1458d36e8 のグローバルインスタンスを返す。初回だけ 0x1427f4bc0 を呼ぶ。
- 0x1427f4bc0: 同じ LCG を回しつつ seed に `this` ポインタ、グローバル値 (0x145c2eaa0)、import 0x14318cda8 (時刻系)、import 0x14318b5f8 (QueryPerformanceCounter 系と思われる) などを XOR していく。
- 0x1427f3290: seed=1 で初期化した上で 0x1427f4bc0 を呼び、上の LCG で 1 byte ずつ `(next >> 16) * 256 >> 32` を作って 16 byte 埋める。
- 0x1427f2ef0: 16 byte をハイフン区切りの 16 進文字列にする。

仮説: これは JUCE の `juce::Random` (`nextInt()`, `getSystemRandom()`, `setSeedRandomly()`) と `juce::Uuid` (`Uuid()`, `toDashedString()`) と思われる。JUCE の実装と定数 (0x5deece66d, +11, 48bit マスク)、`Random::Random() : seed(1) { setSeedRandomly(); }`、`nextInt(256)` の形が一致する。
つまり djmd 系の ID と UUID は暗号論的乱数ではなく、時刻などで seed した 48bit LCG (java.util.Random と同じ式) から作られている可能性が高い。

### 観察: djmdContent (0x141f10ee0、`insertTrack` と推定)

0x141f10ee0 は "UUID", "FolderPath", ... "Reserved1", "djmdContent" を順に参照し、最後に汎用 insert 0x141f491d0 を呼ぶ。ID 生成部分 (0x141f11090〜0x141f110cd):

```
0x141f11090 call 0x1427f4b50        ; getSystemRandom()
0x141f11098 call 0x1427f4b20        ; nextInt()
            mov edi, eax
            test eax, eax      ; je -> 再生成
            cmp eax, -1        ; je -> 再生成
            test eax, 0xf0000000 ; jne -> 再生成 (2 回目のループ側)
```

- 0、0xFFFFFFFF、上位 4bit が立っている値を捨てて引き直す。結果は [1, 2^28) の一様乱数 (棄却法)。
- `>> 4` や `& 0x0FFFFFFF` ではない。100 未満を捨てる下限処理は無い。
- 生成直後に既存 ID を SELECT する処理は見当たらない。
- 生成した ID は 0x1427e9190 (unsigned を 10 進文字列化) で文字列にして "ID" 列に入れる。
- MasterSongID: 0x141f11af9 で `if (track->MasterSongID(+0x40c) == 0) track->MasterSongID = id;`。呼び出し元が値を持っていなければ ID と同じ値になる。
- MasterDBID: `track->+0x408 = controller->+0x54` (DB の DBID と思われる)。
- UUID 列: 0x1427f3290 / 0x1427f2ef0 (juce::Uuid) の値。
- rb_local_usn: 0x141f124df で 0x141f4a300 を 1 回呼んだ戻り値を汎用 insert の第 5 引数に渡す。
- 衝突時: insert の戻り値が 0x13 (SQLITE_CONSTRAINT) のとき、`MasterSongID == id` なら MasterSongID を 0 に戻し、vtable +0x260 の関数を同じ track で呼び直してその結果を返す (0x141f12589〜0x141f125ac)。

仮説: vtable +0x260 は insertTrack 自身で、PRIMARY KEY 衝突時は ID を引き直して再試行していると思われる (Ghidra で vtable を確認する予定)。つまり衝突チェックは「事前 SELECT」ではなく「INSERT 失敗時に再試行」の方式。

### 観察: マスタ系 (Artist / Album / Genre / Key / Label)

| 関数 | テーブル | ID 生成の棄却条件 | 衝突時 |
|---|---|---|---|
| 0x141f0fdc0 | djmdArtist | 0, 0xFFFFFFFF | 0x13 なら vtable +0x218 を呼ぶ |
| 0x141f0fa80 | djmdGenre | 0, 0xFFFFFFFF | 0x13 なら vtable +0x210 を呼ぶ |
| 0x141f10100 | djmdAlbum | 0, 0xFFFFFFFF | 未確認 |
| 0x141f10510 | djmdKey | 0, 0xFFFFFFFF | 未確認 |
| 0x141f10850 | djmdLabel | 0, 0xFFFFFFFF | 未確認 |

- いずれも最初に vtable +0x920 などで既存行を探し、無ければ `nextInt()` で ID を作る。上位 bit の制限は無いので [1, 2^32 - 2] の一様乱数。
- 0x1427e9190 で unsigned として 10 進化するので、DB 上は正の 32bit 値になる。
- UUID 列は juce::Uuid。rb_local_usn は 0x141f4a300 を 1 回。

### 観察: nextInt() の他の呼び出し元 (0x141f... 帯)

`call 0x1427f4b20` の直後の棄却条件を全呼び出し元で見た。

- 0, 0xFFFFFFFF のみ棄却: 0x141efcc60 (djmdColor / djmdMyTag / djmdProperty を参照), 0x141f214b0 (`addActiveCensor`), 0x141f22e60, 0x141f27560, 0x141f281b0 (djmdKey), 0x141f29490 (djmdLabel), 0x141f298f0 (djmdGenre), 0x141f2acb0, 0x141f391d0
- 0, 0xFFFFFFFF に加えて `<= 4` を棄却: 0x141f173c0 (直後に `select * from djmdMyTag where rb_local_deleted = 0` を組み立てる)
- 0, 0xFFFFFFFF に加えて `< 4` を棄却: 0x141f1df00
- 0, 0xFFFFFFFF に加えて `< 3000 (0xbb8)` を棄却: 0x141f39820 (`db::CloudAgentAPI::HistoryIF::addNewHistory`、直後に `select max(Seq) ... from djmdHistory`)
- 0 と 0xFFFFFFFF を棄却するだけの小さい関数: 0x141d44c10, 0x141f2db30, 0x141f2ec80, 0x141f3a5b0 (直接 call は無く、仮想関数と思われる)
- 0x141d44b40: 同じループで ID を作って `[this+8]` に格納し vtable を設定するコンストラクタ風の関数。0x141ecfa40 など 18 箇所から呼ばれる。
- 0x141f7a290 / 0x141f7a6f0: `id == 0 || id == [obj+0x1d0]` の間引き直す (別用途の可能性)

仮説: djmdMyTag は 1〜4 (または 0〜4) を予約、djmdHistory は 3000 未満を予約している可能性がある。どの関数がどのテーブルかは Ghidra で確かめる。

### 観察: djmdMixerParam (0x141f205d0)

- "ID" 列に juce::Uuid の文字列 (0x1427f3290 → 0x1427f2ef0) を入れる。"UUID" 列にはもう 1 回生成した別の juce::Uuid を入れる。
- rb_local_usn は 0x141f4a300 を 1 回。0x13 の再試行は見当たらない。

### 観察: rb_local_usn (localUpdateCount)

- 0x141f4a300 (`nextLocalUsn` と推定): `select int_1 as rb_local_usn from agentRegistry where registry_id = 'localUpdateCount'` を実行し、失敗時は 0xFFFFFFFF を返す。成功時は `usn = int_1 + 1` を 0x141f4abb0 に渡して `update agentRegistry set int_1 = %d ...` を実行し、`usn` を返す。
- 0x141f4abb0 (`setLocalUpdateCount` と推定): 上の update を実行するだけ。
- 汎用 insert 0x141f491d0 は USN を自分で取らず、呼び出し元から第 5 引数でもらって `__RB_LOCAL_USN__` を置換する (0x1427e9290 = signed int の 10 進化)。
- insertTrack, Artist, Genre, MixerParam などは insert 1 行ごとに 0x141f4a300 を 1 回呼ぶ。

仮説: USN は「行ごとに +1 してから使う」方式 (新しい行の rb_local_usn = 更新後の localUpdateCount)。トランザクション単位でまとめる処理は、少なくとも上の関数群には見当たらない。0x141f4a300 の呼び出し元は 28 箇所 (+ pdata 境界の都合で取りこぼしあり) で、Ghidra で一覧化する予定。

## 2026-10-08 調査 1 の続き (capstone)

### 観察: 仮想関数テーブルと RTTI

- 0x141f10ee0 (insertTrack と推定) は `db::AppSyncDBController` の vtable 0x1438e5000 の +0x260 に入っている。よって調査 1 の「衝突時に vtable +0x260 を呼ぶ」は自分自身の再帰呼び出し。
  - 同様に Artist (0x141f0fdc0) は +0x218、Genre (0x141f0fa80) は +0x210 で、衝突時の呼び出し先も自分自身のスロット。
  - 仮説: djmdContent / djmdArtist / djmdGenre は、INSERT が SQLITE_CONSTRAINT で失敗したら関数ごと再実行して新しい乱数 ID を引く。事前の存在チェックは無い。
- 0x141d44b40 は `rb::middleware::db::Id` (RTTI `.?AVId@db@middleware@rb@@`、vtable 0x14388ca58) のコンストラクタで、`nextInt()` を 0 と 0xFFFFFFFF を除いて引き直し `[this+8]` に入れる。新しい DAO 層 (`rb::middleware::db::rekordbox::*Dao` / `*Repository`) の 32bit ID と思われる。
- JUCE を使っていることは RTTI (`juce::String`, `juce::Array` など) からも裏付けられる。

### 観察: 他テーブルの nextInt 呼び出し元 (vtable スロットと参照 RTTI から推定)

| 関数 | AppSyncDBController の slot | 推定テーブル | 棄却条件 |
|---|---|---|---|
| 0x141f173c0 | +0x418 | djmdMyTag (`select * from djmdMyTag ...` を参照) | 0, 0xFFFFFFFF, `<= 4` |
| 0x141f1df00 | +0x618 | djmdSampler (`SqLiteDao<Sampler>` の vtable を参照) | 0, 0xFFFFFFFF, `< 4` |
| 0x141f22e60 | +0x800 | djmdRelatedTracks (`select * from djmdRelatedTracks ...`) | 0, 0xFFFFFFFF |
| 0x141f27560 | +0x998 | djmdHotCueBanklist 系 (`SqLiteDao<HotCueBanklist>` など) | 0, 0xFFFFFFFF |
| 0x141f2acb0 | +0xad8 | djmdHotCueBanklist 系 | 0, 0xFFFFFFFF |
| 0x141f214b0 | +0x720 | djmdActiveCensor (`addActiveCensor` 文字列) | 0, 0xFFFFFFFF |
| 0x141f281b0 / 0x141f29490 / 0x141f298f0 | +0x9d8 / +0xa30 / +0xa38 | djmdKey / djmdLabel / djmdGenre (2 系統目) | 0, 0xFFFFFFFF |
| 0x141efcc60 | 別 vtable 0x1438e4a28 の +0x510 | 初期データ投入 (djmdColor / djmdMyTag の既定値) | 0, 0xFFFFFFFF |
| 0x141f39820 | (非仮想、0x14213ca00 / 0x142148260 から) | djmdHistory (`CloudAgentAPI::HistoryIF::addNewHistory`) | 0, 0xFFFFFFFF, `< 3000` |

仮説: MyTag の 1〜4 は既定の 4 カテゴリ (Genre / Components / Situation / Untitled Column、0x141f173c0 が参照する文字列) 用、Sampler の 1〜3 は固定のルート系、History の 3000 未満は何らかの予約と思われる。いずれも乱数 ID の下限を設けている。

### 観察: djmdSongPlaylist のまとめ追加 (0x141f13e20、slot +0x2e0)

- 先頭で 1 回だけ 0x141f4a300 (nextLocalUsn) を呼び、その値をループ内のすべての行の rb_local_usn に使う。
- ループで行ごとに `insert into djmdSongPlaylist (ID, PlaylistID, ContentID, TrackNo, UUID, rb_data_status, rb_local_deleted, rb_local_usn, rb_local_synced, created_at, updated_at) values (` を組み立てる。
  - ID = juce::Uuid の文字列 (0x1427f3290 → 0x1427f2ef0)
  - PlaylistID = 引数 (unsigned)、ContentID = 配列の i 番目、TrackNo = 開始番号 + i
  - UUID = もう 1 つ別の juce::Uuid
- 失敗時に再試行する分岐は無く、ループを抜ける。

### 観察: djmdSongMyTag の追加 (0x141f41cf0)

- `begin exclusive` の後、行ごとのループの中で ID = juce::Uuid、UUID = juce::Uuid を作り、`select int_1 as rb_local_usn ...` を直接実行して +1 し、0x141f4abb0 で書き戻してから汎用 insert を呼ぶ。行ごとに USN が 1 ずつ進む。

仮説: USN の付け方は関数によって違う。単発 insert (Content, Artist, MixerParam など) と SongMyTag は行ごとに +1、SongPlaylist のまとめ追加は 1 回の操作で同じ USN を共有する。

### 観察: 新しい DAO 層の localUpdateCount

- 0x141fea290 が文字列 "localUpdateCount" (0x14390bb00) を使って agentRegistry 行を取得し、vtable +0x38 を呼んでいる。呼び出し元は 142 箇所 (0x141e88650 〜 0x141fe4350 など)。中身は Ghidra で確認する。

### 観察: rekordboxAgent (参考、Ghidra 対象外)

- `rekordboxAgent-win32-x64\resources\app.asar` の中身は bytenode の V8 バイトコード (.jsc) で、文字列しか読めない。
- `models/model_util.jsc` に `generateRandomIDSet`, `randomBytes`, `generateUnusedID`, `generateUnusedIDs`, `Couldn't generate new ID`, `resolveIDConflictForServerItem`, `changeIDForUUIDIfNeeded`, ` SET ID = ?, rb_local_usn = ? WHERE ID = ?` などがある。
- `models/contentFile.jsc` は `encodeURIComponent` と ContentID / Path / Hash などの列定義を持つ。contentFile の ID (`<UUID>_<URL エンコードしたパス>`) は Agent 側で作られている可能性が高い。
- 仮説: pyrekordbox の「randomBytes 由来の ID + 未使用になるまで再試行」は Agent 側 (Node の crypto.randomBytes) の方式に近い。rekordbox.exe 本体の方式 (JUCE LCG + 棄却法 + INSERT 失敗時に再試行) とは別物と思われる。

## 2026-10-08 調査 2: Ghidra (headless で自動解析 → GUI + MCP)

- Ghidra 12.0.3、プロジェクト: `<scratchpad>\ghidra\rb723.gpr` (`/rekordbox.exe`)。headless 解析は約 55 分 (`-loader-loadLibraries false`)。
  - scratchpad = `C:\Users\naari\AppData\Local\Temp\claude\C--Users-naari-src-github-com-naari3-rkpack\689fd8b3-3958-4169-88e5-d9268f3271ce\scratchpad` (セッション終了で消える可能性あり)
- 付けた関数名 (保存済み、`_guess` は文字列や構造からの推定):
  - 0x141f10ee0 `AppSyncDBController_insertTrack`
  - 0x1427f4b20 `juce_Random_nextInt` / 0x1427f4b50 `juce_Random_getSystemRandom` / 0x1427f4bc0 `juce_Random_setSeedRandomly`
  - 0x1427f3290 `juce_Uuid_ctor` / 0x1427f2ef0 `juce_Uuid_toDashedString`
  - 0x141f4a300 `nextLocalUsn` / 0x141f4abb0 `setLocalUpdateCount` / 0x141f491d0 `buildGenericInsertSql`
  - 0x141d44b40 `middleware_db_Id_ctorRandom` / 0x141d45080 `middleware_db_Uuid_ctorRandom`
  - 0x1418cd650 `cloud_library_sync_PlaylistId_ctorRandom`
  - 0x141fea290 `AgentRegistry_incrementLocalUpdateCount`
  - 0x141f13e20 `AppSyncDBController_addMultiTracksToPlaylist_guess`, 0x141f0fdc0 `..._getAndAddArtistID_guess`, 0x141f0fa80 `..._getAndAddGenreID_guess`, 0x141f391d0 `..._addCue_guess`, 0x141f39820 `CloudAgentAPI_HistoryIF_addNewHistory_guess`, 0x141f04f90 `AppSyncDBController_insertPlaylist_guess`

### 観察: insertTrack のデコンパイル結果 (要約)

```c
do { do { r = juce_Random_nextInt(getSystemRandom()); } while (r == 0); } while (r == 0xffffffff);
while ((r & 0xf0000000) != 0) { /* 同じループで引き直し */ }
ID = String(r) (unsigned); UUID = juce_Uuid_toDashedString(juce_Uuid_ctor());
...
track->MasterDBID(+0x408) = this->+0x54;
if (track->MasterSongID(+0x40c) == 0) track->MasterSongID = r;
...
rb_file_id = FUN_1427c2c00(FUN_142800060(track->path(+0x98)));   // パス由来の 64bit 値と思われる (未確認)
usn = nextLocalUsn();
sql = buildGenericInsertSql("djmdContent", columns, values, usn);
rc = exec(sql);
if (rc == 0)        return r;
else if (rc == 0x13) {  // SQLITE_CONSTRAINT
    if (r == track->MasterSongID) track->MasterSongID = 0;
    return this->vtbl[0x260/8](this, track);   // = insertTrack 自身
}
```

調査 1 の逆アセンブルと同じ内容だった。上位 4bit チェックは棄却ループで、`>> 4` / `& 0x0FFFFFFF` ではない。

### 観察: 新しい DAO 層 (rb::middleware::db::rekordbox)

- エンティティの既定コンストラクタが ID を作る。
  - 32bit 数値 ID のエンティティ: `middleware_db_Id_ctorRandom` (0, 0xFFFFFFFF を棄却する nextInt)。例: 0x141ed28a0 (Category。呼び出し元 0x141f45260 で明示 ID に上書きされる)。
  - Playlist (0x141ee5400): `DbPlaylistId` (0x1420351e0) → `cloud_library_sync_PlaylistId_ctorRandom`。nextInt が [1, 0xFFFFFFFE] なら採用。0 / 0xFFFFFFFF のときに特別 ID (100000, 1000000, FUN_1413857c0 の戻り値) と比べる分岐があるが、結果としてはどれも採用されず引き直しになると読める。
  - UUID 文字列 ID のエンティティ: SongHistory (0x141eeb670)、SongPlaylist (0x141eedeb0) は `middleware_db_Uuid_ctorRandom` (= juce::Uuid の dashed string) を ID に入れる。
- `AppSyncDBController_insertPlaylist_guess` (0x141f04f90): 引数の ID が 0 ならエンティティ既定の乱数 ID、非 0 ならその ID で `PlaylistRepository::SaveNewPlaylist` (0x141ff9990) を呼ぶ。
- `AppSyncDBController_addCue_guess` (0x141f391d0): nextInt (0, 0xFFFFFFFF 棄却) を Cue の ID にして `CueRepository::SaveNewCue` (0x141fd32d0) を呼ぶ。
- `CloudAgentAPI_HistoryIF_addNewHistory_guess` (0x141f39820): `< 3000` も棄却した乱数を djmdHistory の ID にし、`HistoryRepository::SaveNewHistory` (0x141fd8880) を呼ぶ。
- ContentRepository::Save (0x141fd1a30) の唯一の呼び出し元 0x141f09300 はゴミ箱 (TrashContent) からの復元で、既存 ID をそのまま使う。新規 djmdContent を作るのは insertTrack だけと思われる (他に 0xF0000000 を使う箇所は DB 周り 0x141d00000-0x142100000 に無かった)。

### 観察: 新しい DAO 層の rb_local_usn

- `AgentRegistry_incrementLocalUpdateCount` (0x141fea290): agentRegistry の "localUpdateCount" 行を取り、`int_1 + 1` を入れたコピーを作って vtable +0x38 (保存) に渡し、`int_1 + 1` を返す (コピー処理 0x141ed0a60 が `*int_1 + 1` を作る)。旧層の nextLocalUsn と同じ意味。
- 呼び出し元 (142 箇所) は `RepositoryBase<T, ...LibrarySyncSql>` / `SongListRepositoryBase` / `SongTrackRepositoryBase` の保存系メソッド。例: 0x141e88650 は 1 エンティティを保存するたびに 1 回 USN を進めてエンティティに入れる。
- RTTI 上の DAO の型引数:
  - `LibrarySyncSql`: Album, Artist, Category, CloudAgentRegistry, CloudExportPlaylist, CloudExportSongPlaylist, CloudFilterPlaylist, CloudProperty, Color, Content, ContentActiveCensor, ContentCue, Genre, History, HotCueBanklist, HotCueBanklistCue, Key, Label, MenuItems, MixerParam, MyTag, Playlist, RecommendLike, RelatedTracks, Sampler, SettingFile, SongHistory, SongMyTag, SongPlaylist, SongSampler, Sort, TrashContent
  - `NoLibrarySyncSql`: ActiveCensor, AgentRegistry, Cue, Property, SharedPlaylist, SharedPlaylistUser, SongHotCueBanklist, SongTagList, TrashPlaylist, TrashSongPlaylist
- 仮説: `LibrarySyncSql` のテーブルが rb_local_usn を持つ (クラウド同期対象)。djmdCue が `NoLibrarySyncSql` なのは、実 DB で djmdCue の rb_local_usn が全行 NULL だったこと (`notes/id-allocation.md`) と整合する。

### 観察: 28bit の理由の手がかり

- DB 周りでは 0xF0000000 / 0x0FFFFFFF を使うのは insertTrack だけ。ほかの 0x10000000 系の即値は C++ の一時オブジェクト破棄フラグやデバイス名テーブルで、ContentID とは関係が無さそうだった。
- 28bit にする理由は見つからなかった (未解決)。

### 観察: UUID の形式

- `juce_Uuid_ctor` の末尾で byte 8 に `| 0x80` (variant) を立てている。JUCE と同じく byte 6 の version も 4 にしていると思われる (末尾以外は未確認)。小文字 16 進のハイフン区切り。

## 2026-10-08 まとめ (この時点の理解。推論を含む)

1. djmdContent.ID: `AppSyncDBController_insertTrack` (0x141f10ee0) で JUCE の `Random::getSystemRandom().nextInt()` (48bit LCG、時刻などで seed) を 0 / 0xFFFFFFFF / 上位 4bit 非 0 の間引き直す棄却法で作る。範囲は [1, 2^28)。100 未満を捨てる処理は無い。事前の存在チェックは無く、INSERT が SQLITE_CONSTRAINT で失敗したら関数ごと再実行する。パス・ハッシュ・時刻を直接 ID にする処理は無い (seed に時刻などが入るだけ)。MasterSongID は呼び出し元が 0 なら ID と同じ値。
2. 28bit にする理由: 見つからなかった。
3. 他のテーブル:
   - Artist / Album / Genre / Key / Label / ActiveCensor / RelatedTracks / HotCueBanklist / Cue / Playlist / Category など: 32bit 乱数 (0, 0xFFFFFFFF を除く)。
   - MyTag は 5 以上、Sampler は 4 以上、History は 3000 以上 (下限つき)。
   - djmdSongPlaylist / djmdSongMyTag / djmdSongHistory / djmdMixerParam: ID は juce::Uuid の文字列 (UUID 列とは別に生成)。
   - contentFile / contentCue の ID 生成は rekordbox.exe 内では見つからなかった。rekordboxAgent (Node、bytenode) 側で作られている可能性が高い (未確認)。
4. rb_local_usn: agentRegistry 'localUpdateCount' の int_1 を +1 して書き戻し、その値を行に入れる。基本は 1 行 (1 エンティティ) ごとに +1。例外として djmdSongPlaylist のまとめ追加 (0x141f13e20) は 1 回の操作で 1 つの USN を全行に使う。NoLibrarySyncSql の DAO (Cue, SongHotCueBanklist, SongTagList, ActiveCensor, Property など) は USN を付けないと思われる。
