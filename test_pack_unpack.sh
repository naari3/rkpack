#!/usr/bin/env bash
# pack/unpack の統合テスト
#
# 使い方: bash test_pack_unpack.sh [プレイリスト名]
#
# 既定では小さいプレイリストを自動選択する。
# 引数でプレイリスト名を指定することもできる。
#
# NOTE: Windows の sqlite3.exe は main() を使うため、コマンドライン引数が
# ACP (cp932) に変換される。UTF-8 文字列を含む SQL は stdin 経由で渡す。
set -euo pipefail

BIN="cargo run --"
TEST_DIR=".test_pack_unpack_tmp"
DECRYPTED_DB="$TEST_DIR/master_decrypted.db"
PACK_FILE="$TEST_DIR/pack_output.rkp"
DEST_DIR="$TEST_DIR/audio_dest"
DEST_DB="$TEST_DIR/dest.db"

cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

fail() { echo "FAIL: $1" >&2; exit 1; }
pass() { echo "PASS: $1"; }

# sqlite3 を stdin 経由で実行するヘルパー (UTF-8 安全)
sql() { echo "$2" | sqlite3 "$1"; }

# --- 暗号化DBから平文DBをエクスポート ---
echo "--- Export ---"
$BIN export "$DECRYPTED_DB"
[ -f "$DECRYPTED_DB" ] || fail "export に失敗しました"
pass "master_decrypted.db エクスポート完了"

# --- プレイリスト名の決定 ---
if [ $# -ge 1 ]; then
    PLAYLIST="$1"
    SRC_PLAYLIST_ID=$(sql "$DECRYPTED_DB" \
        "SELECT ID FROM djmdPlaylist WHERE Name = '$PLAYLIST' AND rb_local_deleted = 0 LIMIT 1;")
    [ -n "$SRC_PLAYLIST_ID" ] || fail "プレイリスト '$PLAYLIST' が見つかりません"
else
    # 曲数が少ないプレイリストを自動選択 (ID を取得)
    SRC_PLAYLIST_ID=$(sql "$DECRYPTED_DB" "
        SELECT p.ID
        FROM djmdPlaylist p
        JOIN djmdSongPlaylist sp ON sp.PlaylistID = p.ID AND sp.rb_local_deleted = 0
        WHERE p.rb_local_deleted = 0 AND p.Attribute = 0
        GROUP BY p.ID
        HAVING COUNT(*) BETWEEN 1 AND 5
        ORDER BY COUNT(*) ASC
        LIMIT 1;")
    [ -n "$SRC_PLAYLIST_ID" ] || fail "テスト用プレイリストが見つかりません"
    PLAYLIST=$(sql "$DECRYPTED_DB" "SELECT Name FROM djmdPlaylist WHERE ID = '$SRC_PLAYLIST_ID';")
fi

TRACK_COUNT=$(sql "$DECRYPTED_DB" \
    "SELECT COUNT(*) FROM djmdSongPlaylist WHERE PlaylistID = '$SRC_PLAYLIST_ID' AND rb_local_deleted = 0;")
SRC_DBID=$(sql "$DECRYPTED_DB" "SELECT DBID FROM djmdProperty LIMIT 1;")
SRC_DEVICE_ID=$(sql "$DECRYPTED_DB" "SELECT ID FROM djmdDevice WHERE rb_local_deleted = 0 LIMIT 1;")

echo "=== テスト対象プレイリスト: $PLAYLIST (ID: $SRC_PLAYLIST_ID) ==="
echo "ソース: トラック数=$TRACK_COUNT, DBID=$SRC_DBID, DeviceID=$SRC_DEVICE_ID"

# --- 1. Pack ---
echo ""
echo "--- Pack ---"
$BIN --db-path "$DECRYPTED_DB" pack "$PACK_FILE" --playlist "$PLAYLIST"

[ -f "$PACK_FILE" ] || fail ".rkp ファイルが生成されていない"
pass ".rkp ファイル生成"

# .rkp 内に hotCueBanklistCue キーが含まれること
unzip -p "$PACK_FILE" pack.json | jq -e '.tables | has("hotCueBanklistCue")' > /dev/null \
    || fail "hotCueBanklistCue が pack.json に含まれていない"
pass "hotCueBanklistCue in pack.json"

# --- 2. 空の移行先DBを作成 ---
echo ""
echo "--- 移行先 DB 準備 ---"
DEST_DBID="123456789"
DEST_DEVICE="dest-device-test-id"

sql "$DECRYPTED_DB" ".schema" | grep -v sqlite_sequence | sqlite3 "$DEST_DB"

sql "$DEST_DB" "
INSERT INTO djmdProperty (DBID, DBVersion, created_at, updated_at)
VALUES ('$DEST_DBID', '6000', datetime('now'), datetime('now'));
INSERT INTO djmdDevice (ID, MasterDBID, Name, created_at, updated_at)
VALUES ('$DEST_DEVICE', '$DEST_DBID', 'TestPC', datetime('now'), datetime('now'));
"
pass "移行先 DB 作成 (DBID=$DEST_DBID, DeviceID=$DEST_DEVICE)"

# --- 3. Unpack ---
echo ""
echo "--- Unpack ---"
$BIN --db-path "$DEST_DB" unpack "$PACK_FILE" --dest-dir "$DEST_DIR"

# --- 4. 検証 ---
echo ""
echo "--- 検証 ---"

# 4-1. トラック数
DEST_TRACK_COUNT=$(sql "$DEST_DB" "SELECT COUNT(*) FROM djmdContent WHERE rb_local_deleted = 0;")
[ "$DEST_TRACK_COUNT" -eq "$TRACK_COUNT" ] \
    || fail "トラック数不一致: 期待=$TRACK_COUNT, 実際=$DEST_TRACK_COUNT"
pass "トラック数一致 ($DEST_TRACK_COUNT)"

# 4-2. MasterDBID
BAD_DBID=$(sql "$DEST_DB" "SELECT COUNT(*) FROM djmdContent WHERE MasterDBID <> '$DEST_DBID';")
[ "$BAD_DBID" -eq 0 ] || fail "MasterDBID 不一致: $BAD_DBID 件"
pass "MasterDBID = $DEST_DBID"

# 4-3. DeviceID
BAD_DEVICE=$(sql "$DEST_DB" "SELECT COUNT(*) FROM djmdContent WHERE DeviceID <> '$DEST_DEVICE';")
[ "$BAD_DEVICE" -eq 0 ] || fail "DeviceID 不一致: $BAD_DEVICE 件"
pass "DeviceID = $DEST_DEVICE"

# 4-4. MasterSongID = ID
BAD_MSID=$(sql "$DEST_DB" "SELECT COUNT(*) FROM djmdContent WHERE MasterSongID <> ID;")
[ "$BAD_MSID" -eq 0 ] || fail "MasterSongID <> ID: $BAD_MSID 件"
pass "MasterSongID = ID"

# 4-5. クラウド同期フィールドリセット (djmdContent)
BAD_SYNC=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM djmdContent
    WHERE rb_data_status <> 0 OR rb_local_data_status <> 0 OR rb_local_synced <> 0
       OR usn IS NOT NULL OR rb_local_usn IS NOT NULL;")
[ "$BAD_SYNC" -eq 0 ] || fail "djmdContent 同期フィールド未リセット: $BAD_SYNC 件"
pass "djmdContent 同期フィールドリセット済み"

# 4-6. djmdCue 同期フィールドリセット
BAD_CUE_SYNC=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM djmdCue WHERE usn IS NOT NULL OR rb_local_usn IS NOT NULL;")
[ "$BAD_CUE_SYNC" -eq 0 ] || fail "djmdCue 同期フィールド未リセット: $BAD_CUE_SYNC 件"
pass "djmdCue 同期フィールドリセット済み"

# 4-7. contentCue JSON 内 ID リマップ (sqlite3 の json_each で検証)
BAD_CUE_CONTENT=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM contentCue cc, json_each(cc.Cues) je
    WHERE json_extract(je.value, '$.ContentID') IS NOT NULL
      AND json_extract(je.value, '$.ContentID') NOT IN (SELECT ID FROM djmdContent);")
[ "$BAD_CUE_CONTENT" -eq 0 ] || fail "contentCue JSON ContentID 未リマップ: $BAD_CUE_CONTENT 件"
BAD_CUE_ID=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM contentCue cc, json_each(cc.Cues) je
    WHERE json_extract(je.value, '$.ID') IS NOT NULL
      AND json_extract(je.value, '$.ID') NOT IN (SELECT ID FROM djmdCue);")
[ "$BAD_CUE_ID" -eq 0 ] || fail "contentCue JSON ID 未リマップ: $BAD_CUE_ID 件"
pass "contentCue JSON ID remapped"

# 4-8. プレイリスト挿入 (日本語名を WHERE で使うので stdin 経由)
PLAYLIST_COUNT=$(sql "$DEST_DB" \
    "SELECT COUNT(*) FROM djmdPlaylist WHERE Name = '$PLAYLIST' AND rb_local_deleted = 0;")
[ "$PLAYLIST_COUNT" -eq 1 ] || fail "プレイリスト未挿入 (count=$PLAYLIST_COUNT)"
pass "プレイリスト挿入済み"

# 4-9. djmdSongPlaylist 数
DEST_SP_COUNT=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM djmdSongPlaylist sp
    JOIN djmdPlaylist p ON p.ID = sp.PlaylistID
    WHERE p.Name = '$PLAYLIST' AND sp.rb_local_deleted = 0;")
[ "$DEST_SP_COUNT" -eq "$TRACK_COUNT" ] \
    || fail "djmdSongPlaylist 数不一致: 期待=$TRACK_COUNT, 実際=$DEST_SP_COUNT"
pass "djmdSongPlaylist 数一致 ($DEST_SP_COUNT)"

# 4-10. FK 整合性: djmdSongPlaylist.ContentID → djmdContent
BROKEN_FK=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM djmdSongPlaylist sp
    WHERE sp.rb_local_deleted = 0
      AND sp.ContentID NOT IN (SELECT ID FROM djmdContent);")
[ "$BROKEN_FK" -eq 0 ] || fail "djmdSongPlaylist FK 破損: $BROKEN_FK 件"
pass "FK 整合性: djmdSongPlaylist -> djmdContent"

# 4-11. contentFile 同期フィールドリセット
BAD_CF_SYNC=$(sql "$DEST_DB" "
    SELECT COUNT(*) FROM contentFile
    WHERE usn IS NOT NULL OR rb_local_usn IS NOT NULL OR rb_insync_local_usn IS NOT NULL;")
[ "$BAD_CF_SYNC" -eq 0 ] || fail "contentFile 同期フィールド未リセット: $BAD_CF_SYNC 件"
pass "contentFile 同期フィールドリセット済み"

# 4-12. 音声ファイル配置
AUDIO_FILE_COUNT=$(find "$DEST_DIR" -type f 2>/dev/null | wc -l | tr -d ' ')
[ "$AUDIO_FILE_COUNT" -ge 1 ] || fail "音声ファイルが配置されていない"
pass "音声ファイル配置済み ($AUDIO_FILE_COUNT 件)"

echo ""
echo "=== 全テスト合格 ==="
