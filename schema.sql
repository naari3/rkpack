DB: C:\Users\naari\AppData\Roaming\Pioneer\rekordbox\master.db
-- agentNotification : アプリ内通知。バナーやOS通知の表示期間・カテゴリ・本文等を管理
CREATE TABLE `agentNotification` (
  `ID` BIGINT PRIMARY KEY,
  `graphic_area` TINYINT(1) DEFAULT 0,
  `text_area` TINYINT(1) DEFAULT 0,
  `os_notification` TINYINT(1) DEFAULT 0,
  `start_datetime` DATETIME DEFAULT NULL,
  `end_datetime` DATETIME DEFAULT NULL,
  `display_datetime` DATETIME DEFAULT NULL,
  `interval` INTEGER DEFAULT 0,
  `category` VARCHAR(255) DEFAULT NULL,
  `category_color` VARCHAR(255) DEFAULT NULL,
  `title` TEXT DEFAULT NULL,
  `description` TEXT DEFAULT NULL,
  `url` VARCHAR(255) DEFAULT NULL,
  `image` VARCHAR(255) DEFAULT NULL,
  `image_path` VARCHAR(255) DEFAULT NULL,
  `read_status` INTEGER DEFAULT 0,
  `last_displayed_datetime` DATETIME DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: agent_notification_start_datetime_end_datetime
CREATE INDEX `agent_notification_start_datetime_end_datetime` ON `agentNotification` (`start_datetime`, `end_datetime`);

-- agentNotificationLog : 通知の表示・クリック等のイベントログ。分析レポート用
CREATE TABLE `agentNotificationLog` (
  `ID` INTEGER PRIMARY KEY AUTOINCREMENT,
  `gigya_uid` VARCHAR(255) DEFAULT NULL,
  `event_date` INTEGER DEFAULT NULL,
  `reported_datetime` DATETIME DEFAULT NULL,
  `kind` INTEGER DEFAULT NULL,
  `value` INTEGER DEFAULT NULL,
  `notification_id` BIGINT DEFAULT NULL,
  `link` VARCHAR(255) DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: agent_notification_log_gigya_uid_event_date_kind_notification_id
CREATE INDEX `agent_notification_log_gigya_uid_event_date_kind_notification_id` ON `agentNotificationLog` (`gigya_uid`, `event_date`, `kind`, `notification_id`);
-- index: agent_notification_log_reported_datetime_event_date
CREATE INDEX `agent_notification_log_reported_datetime_event_date` ON `agentNotificationLog` (`reported_datetime`, `event_date`);

-- agentRegistry : エージェント機能の汎用キーバリューストア
CREATE TABLE `agentRegistry` (
  `registry_id` VARCHAR(255) PRIMARY KEY,
  `id_1` VARCHAR(255) DEFAULT NULL,
  `id_2` VARCHAR(255) DEFAULT NULL,
  `int_1` BIGINT DEFAULT NULL,
  `int_2` BIGINT DEFAULT NULL,
  `str_1` VARCHAR(255) DEFAULT NULL,
  `str_2` VARCHAR(255) DEFAULT NULL,
  `date_1` DATETIME DEFAULT NULL,
  `date_2` DATETIME DEFAULT NULL,
  `text_1` TEXT DEFAULT NULL,
  `text_2` TEXT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: agent_registry_id_1_id_2
CREATE INDEX `agent_registry_id_1_id_2` ON `agentRegistry` (`id_1`, `id_2`);

-- cloudAgentRegistry : クラウド同期対応版の汎用キーバリューストア
CREATE TABLE `cloudAgentRegistry` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `int_1` BIGINT DEFAULT NULL,
  `int_2` BIGINT DEFAULT NULL,
  `str_1` VARCHAR(255) DEFAULT NULL,
  `str_2` VARCHAR(255) DEFAULT NULL,
  `date_1` DATETIME DEFAULT NULL,
  `date_2` DATETIME DEFAULT NULL,
  `text_1` TEXT DEFAULT NULL,
  `text_2` TEXT DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: cloud_agent_registry__u_u_i_d
CREATE INDEX `cloud_agent_registry__u_u_i_d` ON `cloudAgentRegistry` (`UUID`);
-- index: cloud_agent_registry_rb_data_status
CREATE INDEX `cloud_agent_registry_rb_data_status` ON `cloudAgentRegistry` (`rb_data_status`);
-- index: cloud_agent_registry_rb_local_data_status
CREATE INDEX `cloud_agent_registry_rb_local_data_status` ON `cloudAgentRegistry` (`rb_local_data_status`);
-- index: cloud_agent_registry_rb_local_deleted
CREATE INDEX `cloud_agent_registry_rb_local_deleted` ON `cloudAgentRegistry` (`rb_local_deleted`);
-- index: cloud_agent_registry_rb_local_usn__i_d
CREATE INDEX `cloud_agent_registry_rb_local_usn__i_d` ON `cloudAgentRegistry` (`rb_local_usn`, `ID`);

-- contentActiveCensor : 楽曲のアクティブセンサー情報（自動ミュート区間等）をJSON的に保持
CREATE TABLE `contentActiveCensor` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `ActiveCensors` TEXT DEFAULT NULL,
  `rb_activecensor_count` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: content_active_censor__content_i_d
CREATE INDEX `content_active_censor__content_i_d` ON `contentActiveCensor` (`ContentID`);
-- index: content_active_censor__u_u_i_d
CREATE INDEX `content_active_censor__u_u_i_d` ON `contentActiveCensor` (`UUID`);
-- index: content_active_censor_rb_data_status
CREATE INDEX `content_active_censor_rb_data_status` ON `contentActiveCensor` (`rb_data_status`);
-- index: content_active_censor_rb_local_data_status
CREATE INDEX `content_active_censor_rb_local_data_status` ON `contentActiveCensor` (`rb_local_data_status`);
-- index: content_active_censor_rb_local_deleted
CREATE INDEX `content_active_censor_rb_local_deleted` ON `contentActiveCensor` (`rb_local_deleted`);
-- index: content_active_censor_rb_local_usn__i_d
CREATE INDEX `content_active_censor_rb_local_usn__i_d` ON `contentActiveCensor` (`rb_local_usn`, `ID`);

-- contentCue : 楽曲のキューポイント情報をまとめてJSON的に保持（djmdContent対応）
CREATE TABLE `contentCue` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `Cues` TEXT DEFAULT NULL,
  `rb_cue_count` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: content_cue__content_i_d
CREATE INDEX `content_cue__content_i_d` ON `contentCue` (`ContentID`);
-- index: content_cue__u_u_i_d
CREATE INDEX `content_cue__u_u_i_d` ON `contentCue` (`UUID`);
-- index: content_cue_rb_cue_count
CREATE INDEX `content_cue_rb_cue_count` ON `contentCue` (`rb_cue_count`);
-- index: content_cue_rb_data_status
CREATE INDEX `content_cue_rb_data_status` ON `contentCue` (`rb_data_status`);
-- index: content_cue_rb_local_data_status
CREATE INDEX `content_cue_rb_local_data_status` ON `contentCue` (`rb_local_data_status`);
-- index: content_cue_rb_local_deleted
CREATE INDEX `content_cue_rb_local_deleted` ON `contentCue` (`rb_local_deleted`);
-- index: content_cue_rb_local_usn__i_d
CREATE INDEX `content_cue_rb_local_usn__i_d` ON `contentCue` (`rb_local_usn`, `ID`);

-- contentFile : 楽曲の分析データファイル（.dat/.ext/.2ex）。クラウド同期のパス・ハッシュ・進捗状態を管理
CREATE TABLE `contentFile` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `Path` VARCHAR(255) DEFAULT NULL,
  `Hash` VARCHAR(255) DEFAULT NULL,
  `Size` INTEGER DEFAULT NULL,
  `rb_local_path` VARCHAR(255) DEFAULT NULL,
  `rb_insync_hash` VARCHAR(255) DEFAULT NULL,
  `rb_insync_local_usn` BIGINT DEFAULT NULL,
  `rb_file_hash_dirty` INTEGER DEFAULT 0,
  `rb_local_file_status` INTEGER DEFAULT 0,
  `rb_in_progress` TINYINT(1) DEFAULT 0,
  `rb_process_type` INTEGER DEFAULT 0,
  `rb_temp_path` VARCHAR(255) DEFAULT NULL,
  `rb_priority` INTEGER DEFAULT 50,
  `rb_file_size_dirty` INTEGER DEFAULT 0,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: content_file__content_i_d
CREATE INDEX `content_file__content_i_d` ON `contentFile` (`ContentID`);
-- index: content_file__u_u_i_d
CREATE INDEX `content_file__u_u_i_d` ON `contentFile` (`UUID`);
-- index: content_file_rb_data_status
CREATE INDEX `content_file_rb_data_status` ON `contentFile` (`rb_data_status`);
-- index: content_file_rb_file_hash_dirty
CREATE INDEX `content_file_rb_file_hash_dirty` ON `contentFile` (`rb_file_hash_dirty`);
-- index: content_file_rb_file_size_dirty
CREATE INDEX `content_file_rb_file_size_dirty` ON `contentFile` (`rb_file_size_dirty`);
-- index: content_file_rb_local_data_status
CREATE INDEX `content_file_rb_local_data_status` ON `contentFile` (`rb_local_data_status`);
-- index: content_file_rb_local_deleted
CREATE INDEX `content_file_rb_local_deleted` ON `contentFile` (`rb_local_deleted`);
-- index: content_file_rb_local_deleted_rb_in_progress_rb_local_file_status_rb_process_type_rb_priority
CREATE INDEX `content_file_rb_local_deleted_rb_in_progress_rb_local_file_status_rb_process_type_rb_priority` ON `contentFile` (`rb_local_deleted`, `rb_in_progress`, `rb_local_file_status`, `rb_process_type`, `rb_priority`);
-- index: content_file_rb_local_usn__i_d
CREATE INDEX `content_file_rb_local_usn__i_d` ON `contentFile` (`rb_local_usn`, `ID`);

-- djmdActiveCensor : 楽曲ごとのアクティブセンサー（自動ミュート）区間。InMsec/OutMsecで範囲指定
CREATE TABLE `djmdActiveCensor` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `InMsec` INTEGER DEFAULT NULL,
  `OutMsec` INTEGER DEFAULT NULL,
  `Info` INTEGER DEFAULT NULL,
  `ParameterList` TEXT DEFAULT NULL,
  `ContentUUID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_active_censor__content_i_d
CREATE INDEX `djmd_active_censor__content_i_d` ON `djmdActiveCensor` (`ContentID`);
-- index: djmd_active_censor__content_u_u_i_d
CREATE INDEX `djmd_active_censor__content_u_u_i_d` ON `djmdActiveCensor` (`ContentUUID`);
-- index: djmd_active_censor__u_u_i_d
CREATE INDEX `djmd_active_censor__u_u_i_d` ON `djmdActiveCensor` (`UUID`);
-- index: djmd_active_censor_rb_data_status
CREATE INDEX `djmd_active_censor_rb_data_status` ON `djmdActiveCensor` (`rb_data_status`);
-- index: djmd_active_censor_rb_local_data_status
CREATE INDEX `djmd_active_censor_rb_local_data_status` ON `djmdActiveCensor` (`rb_local_data_status`);
-- index: djmd_active_censor_rb_local_deleted
CREATE INDEX `djmd_active_censor_rb_local_deleted` ON `djmdActiveCensor` (`rb_local_deleted`);
-- index: djmd_active_censor_rb_local_usn__i_d
CREATE INDEX `djmd_active_censor_rb_local_usn__i_d` ON `djmdActiveCensor` (`rb_local_usn`, `ID`);

-- djmdAlbum : アルバムマスタ。名前・アルバムアーティスト・ジャケット画像パス
CREATE TABLE `djmdAlbum` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Name` VARCHAR(255) DEFAULT NULL,
  `AlbumArtistID` VARCHAR(255) DEFAULT NULL,
  `ImagePath` VARCHAR(255) DEFAULT NULL,
  `Compilation` INTEGER DEFAULT NULL,
  `SearchStr` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_album__album_artist_i_d
CREATE INDEX `djmd_album__album_artist_i_d` ON `djmdAlbum` (`AlbumArtistID`);
-- index: djmd_album__name
CREATE INDEX `djmd_album__name` ON `djmdAlbum` (`Name`);
-- index: djmd_album__u_u_i_d
CREATE INDEX `djmd_album__u_u_i_d` ON `djmdAlbum` (`UUID`);
-- index: djmd_album_rb_data_status
CREATE INDEX `djmd_album_rb_data_status` ON `djmdAlbum` (`rb_data_status`);
-- index: djmd_album_rb_local_data_status
CREATE INDEX `djmd_album_rb_local_data_status` ON `djmdAlbum` (`rb_local_data_status`);
-- index: djmd_album_rb_local_deleted
CREATE INDEX `djmd_album_rb_local_deleted` ON `djmdAlbum` (`rb_local_deleted`);
-- index: djmd_album_rb_local_usn__i_d
CREATE INDEX `djmd_album_rb_local_usn__i_d` ON `djmdAlbum` (`rb_local_usn`, `ID`);

-- djmdArtist : アーティストマスタ。名前と検索用文字列
CREATE TABLE `djmdArtist` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Name` VARCHAR(255) DEFAULT NULL,
  `SearchStr` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_artist__name
CREATE INDEX `djmd_artist__name` ON `djmdArtist` (`Name`);
-- index: djmd_artist__u_u_i_d
CREATE INDEX `djmd_artist__u_u_i_d` ON `djmdArtist` (`UUID`);
-- index: djmd_artist_rb_data_status
CREATE INDEX `djmd_artist_rb_data_status` ON `djmdArtist` (`rb_data_status`);
-- index: djmd_artist_rb_local_data_status
CREATE INDEX `djmd_artist_rb_local_data_status` ON `djmdArtist` (`rb_local_data_status`);
-- index: djmd_artist_rb_local_deleted
CREATE INDEX `djmd_artist_rb_local_deleted` ON `djmdArtist` (`rb_local_deleted`);
-- index: djmd_artist_rb_local_usn__i_d
CREATE INDEX `djmd_artist_rb_local_usn__i_d` ON `djmdArtist` (`rb_local_usn`, `ID`);

-- djmdCategory : ブラウズ画面のカテゴリ（ジャンル/アーティスト等）の表示順・有効無効
CREATE TABLE `djmdCategory` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `MenuItemID` VARCHAR(255) DEFAULT NULL,
  `Seq` INTEGER DEFAULT NULL,
  `Disable` INTEGER DEFAULT NULL,
  `InfoOrder` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_category__u_u_i_d
CREATE INDEX `djmd_category__u_u_i_d` ON `djmdCategory` (`UUID`);
-- index: djmd_category_rb_data_status
CREATE INDEX `djmd_category_rb_data_status` ON `djmdCategory` (`rb_data_status`);
-- index: djmd_category_rb_local_data_status
CREATE INDEX `djmd_category_rb_local_data_status` ON `djmdCategory` (`rb_local_data_status`);
-- index: djmd_category_rb_local_deleted
CREATE INDEX `djmd_category_rb_local_deleted` ON `djmdCategory` (`rb_local_deleted`);
-- index: djmd_category_rb_local_usn__i_d
CREATE INDEX `djmd_category_rb_local_usn__i_d` ON `djmdCategory` (`rb_local_usn`, `ID`);

-- djmdCloudProperty : クラウド同期のプロパティ情報。Reserved列のみで将来拡張用
CREATE TABLE `djmdCloudProperty` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Reserved1` TEXT DEFAULT NULL,
  `Reserved2` TEXT DEFAULT NULL,
  `Reserved3` TEXT DEFAULT NULL,
  `Reserved4` TEXT DEFAULT NULL,
  `Reserved5` TEXT DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_cloud_property__u_u_i_d
CREATE INDEX `djmd_cloud_property__u_u_i_d` ON `djmdCloudProperty` (`UUID`);
-- index: djmd_cloud_property_rb_data_status
CREATE INDEX `djmd_cloud_property_rb_data_status` ON `djmdCloudProperty` (`rb_data_status`);
-- index: djmd_cloud_property_rb_local_data_status
CREATE INDEX `djmd_cloud_property_rb_local_data_status` ON `djmdCloudProperty` (`rb_local_data_status`);
-- index: djmd_cloud_property_rb_local_deleted
CREATE INDEX `djmd_cloud_property_rb_local_deleted` ON `djmdCloudProperty` (`rb_local_deleted`);
-- index: djmd_cloud_property_rb_local_usn__i_d
CREATE INDEX `djmd_cloud_property_rb_local_usn__i_d` ON `djmdCloudProperty` (`rb_local_usn`, `ID`);

-- djmdColor : トラックに割り当てるカラーラベルのマスタ（赤・青等）
CREATE TABLE `djmdColor` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ColorCode` INTEGER DEFAULT NULL,
  `SortKey` INTEGER DEFAULT NULL,
  `Commnt` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_color__u_u_i_d
CREATE INDEX `djmd_color__u_u_i_d` ON `djmdColor` (`UUID`);
-- index: djmd_color_rb_data_status
CREATE INDEX `djmd_color_rb_data_status` ON `djmdColor` (`rb_data_status`);
-- index: djmd_color_rb_local_data_status
CREATE INDEX `djmd_color_rb_local_data_status` ON `djmdColor` (`rb_local_data_status`);
-- index: djmd_color_rb_local_deleted
CREATE INDEX `djmd_color_rb_local_deleted` ON `djmdColor` (`rb_local_deleted`);
-- index: djmd_color_rb_local_usn__i_d
CREATE INDEX `djmd_color_rb_local_usn__i_d` ON `djmdColor` (`rb_local_usn`, `ID`);

-- djmdContent : 楽曲メインテーブル。タイトル・BPM・キー・レーティング・ファイルパス等、全トラック情報の中心
CREATE TABLE `djmdContent` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `FolderPath` VARCHAR(255) DEFAULT NULL,
  `FileNameL` VARCHAR(255) DEFAULT NULL,
  `FileNameS` VARCHAR(255) DEFAULT NULL,
  `Title` VARCHAR(255) DEFAULT NULL,
  `ArtistID` VARCHAR(255) DEFAULT NULL,
  `AlbumID` VARCHAR(255) DEFAULT NULL,
  `GenreID` VARCHAR(255) DEFAULT NULL,
  `BPM` INTEGER DEFAULT NULL,
  `Length` INTEGER DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `BitRate` INTEGER DEFAULT NULL,
  `BitDepth` INTEGER DEFAULT NULL,
  `Commnt` TEXT DEFAULT NULL,
  `FileType` INTEGER DEFAULT NULL,
  `Rating` INTEGER DEFAULT NULL,
  `ReleaseYear` INTEGER DEFAULT NULL,
  `RemixerID` VARCHAR(255) DEFAULT NULL,
  `LabelID` VARCHAR(255) DEFAULT NULL,
  `OrgArtistID` VARCHAR(255) DEFAULT NULL,
  `KeyID` VARCHAR(255) DEFAULT NULL,
  `StockDate` VARCHAR(255) DEFAULT NULL,
  `ColorID` VARCHAR(255) DEFAULT NULL,
  `DJPlayCount` INTEGER DEFAULT NULL,
  `ImagePath` VARCHAR(255) DEFAULT NULL,
  `MasterDBID` VARCHAR(255) DEFAULT NULL,
  `MasterSongID` VARCHAR(255) DEFAULT NULL,
  `AnalysisDataPath` VARCHAR(255) DEFAULT NULL,
  `SearchStr` VARCHAR(255) DEFAULT NULL,
  `FileSize` INTEGER DEFAULT NULL,
  `DiscNo` INTEGER DEFAULT NULL,
  `ComposerID` VARCHAR(255) DEFAULT NULL,
  `Subtitle` VARCHAR(255) DEFAULT NULL,
  `SampleRate` INTEGER DEFAULT NULL,
  `DisableQuantize` INTEGER DEFAULT NULL,
  `Analysed` INTEGER DEFAULT NULL,
  `ReleaseDate` VARCHAR(255) DEFAULT NULL,
  `DateCreated` VARCHAR(255) DEFAULT NULL,
  `ContentLink` INTEGER DEFAULT NULL,
  `Tag` VARCHAR(255) DEFAULT NULL,
  `ModifiedByRBM` VARCHAR(255) DEFAULT NULL,
  `HotCueAutoLoad` VARCHAR(255) DEFAULT NULL,
  `DeliveryControl` VARCHAR(255) DEFAULT NULL,
  `DeliveryComment` VARCHAR(255) DEFAULT NULL,
  `CueUpdated` VARCHAR(255) DEFAULT NULL,
  `AnalysisUpdated` VARCHAR(255) DEFAULT NULL,
  `TrackInfoUpdated` VARCHAR(255) DEFAULT NULL,
  `Lyricist` VARCHAR(255) DEFAULT NULL,
  `ISRC` VARCHAR(255) DEFAULT NULL,
  `SamplerTrackInfo` INTEGER DEFAULT NULL,
  `SamplerPlayOffset` INTEGER DEFAULT NULL,
  `SamplerGain` FLOAT DEFAULT NULL,
  `VideoAssociate` VARCHAR(255) DEFAULT NULL,
  `LyricStatus` INTEGER DEFAULT NULL,
  `ServiceID` INTEGER DEFAULT NULL,
  `OrgFolderPath` VARCHAR(255) DEFAULT NULL,
  `Reserved1` TEXT DEFAULT NULL,
  `Reserved2` TEXT DEFAULT NULL,
  `Reserved3` TEXT DEFAULT NULL,
  `Reserved4` TEXT DEFAULT NULL,
  `ExtInfo` TEXT DEFAULT NULL,
  `rb_file_id` VARCHAR(255) DEFAULT NULL,
  `DeviceID` VARCHAR(255) DEFAULT NULL,
  `rb_LocalFolderPath` VARCHAR(255) DEFAULT NULL,
  `SrcID` VARCHAR(255) DEFAULT NULL,
  `SrcTitle` VARCHAR(255) DEFAULT NULL,
  `SrcArtistName` VARCHAR(255) DEFAULT NULL,
  `SrcAlbumName` VARCHAR(255) DEFAULT NULL,
  `SrcLength` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_content__album_i_d
CREATE INDEX `djmd_content__album_i_d` ON `djmdContent` (`AlbumID`);
-- index: djmd_content__artist_i_d
CREATE INDEX `djmd_content__artist_i_d` ON `djmdContent` (`ArtistID`);
-- index: djmd_content__composer_i_d
CREATE INDEX `djmd_content__composer_i_d` ON `djmdContent` (`ComposerID`);
-- index: djmd_content__genre_i_d
CREATE INDEX `djmd_content__genre_i_d` ON `djmdContent` (`GenreID`);
-- index: djmd_content__key_i_d
CREATE INDEX `djmd_content__key_i_d` ON `djmdContent` (`KeyID`);
-- index: djmd_content__label_i_d
CREATE INDEX `djmd_content__label_i_d` ON `djmdContent` (`LabelID`);
-- index: djmd_content__master_d_b_i_d__master_song_i_d
CREATE INDEX `djmd_content__master_d_b_i_d__master_song_i_d` ON `djmdContent` (`MasterDBID`, `MasterSongID`);
-- index: djmd_content__org_artist_i_d
CREATE INDEX `djmd_content__org_artist_i_d` ON `djmdContent` (`OrgArtistID`);
-- index: djmd_content__remixer_i_d
CREATE INDEX `djmd_content__remixer_i_d` ON `djmdContent` (`RemixerID`);
-- index: djmd_content__u_u_i_d
CREATE INDEX `djmd_content__u_u_i_d` ON `djmdContent` (`UUID`);
-- index: djmd_content_rb_data_status
CREATE INDEX `djmd_content_rb_data_status` ON `djmdContent` (`rb_data_status`);
-- index: djmd_content_rb_local_data_status
CREATE INDEX `djmd_content_rb_local_data_status` ON `djmdContent` (`rb_local_data_status`);
-- index: djmd_content_rb_local_deleted
CREATE INDEX `djmd_content_rb_local_deleted` ON `djmdContent` (`rb_local_deleted`);
-- index: djmd_content_rb_local_deleted__bit_depth
CREATE INDEX `djmd_content_rb_local_deleted__bit_depth` ON `djmdContent` (`rb_local_deleted`, `BitDepth`);
-- index: djmd_content_rb_local_deleted__bit_rate
CREATE INDEX `djmd_content_rb_local_deleted__bit_rate` ON `djmdContent` (`rb_local_deleted`, `BitRate`);
-- index: djmd_content_rb_local_deleted__file_type
CREATE INDEX `djmd_content_rb_local_deleted__file_type` ON `djmdContent` (`rb_local_deleted`, `FileType`);
-- index: djmd_content_rb_local_deleted__service_i_d
CREATE INDEX `djmd_content_rb_local_deleted__service_i_d` ON `djmdContent` (`rb_local_deleted`, `ServiceID`);
-- index: djmd_content_rb_local_usn__i_d
CREATE INDEX `djmd_content_rb_local_usn__i_d` ON `djmdContent` (`rb_local_usn`, `ID`);

-- djmdCue : 楽曲ごとのキュー/ループポイント。位置(msec/frame)・種類・色・コメント
CREATE TABLE `djmdCue` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `InMsec` INTEGER DEFAULT NULL,
  `InFrame` INTEGER DEFAULT NULL,
  `InMpegFrame` INTEGER DEFAULT NULL,
  `InMpegAbs` INTEGER DEFAULT NULL,
  `OutMsec` INTEGER DEFAULT NULL,
  `OutFrame` INTEGER DEFAULT NULL,
  `OutMpegFrame` INTEGER DEFAULT NULL,
  `OutMpegAbs` INTEGER DEFAULT NULL,
  `Kind` INTEGER DEFAULT NULL,
  `Color` INTEGER DEFAULT NULL,
  `ColorTableIndex` INTEGER DEFAULT NULL,
  `ActiveLoop` INTEGER DEFAULT NULL,
  `Comment` VARCHAR(255) DEFAULT NULL,
  `BeatLoopSize` INTEGER DEFAULT NULL,
  `CueMicrosec` INTEGER DEFAULT NULL,
  `InPointSeekInfo` VARCHAR(255) DEFAULT NULL,
  `OutPointSeekInfo` VARCHAR(255) DEFAULT NULL,
  `ContentUUID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_cue__content_i_d_rb_local_deleted
CREATE INDEX `djmd_cue__content_i_d_rb_local_deleted` ON `djmdCue` (`ContentID`, `rb_local_deleted`);
-- index: djmd_cue__content_u_u_i_d
CREATE INDEX `djmd_cue__content_u_u_i_d` ON `djmdCue` (`ContentUUID`);
-- index: djmd_cue__u_u_i_d
CREATE INDEX `djmd_cue__u_u_i_d` ON `djmdCue` (`UUID`);
-- index: djmd_cue_rb_data_status
CREATE INDEX `djmd_cue_rb_data_status` ON `djmdCue` (`rb_data_status`);
-- index: djmd_cue_rb_local_data_status
CREATE INDEX `djmd_cue_rb_local_data_status` ON `djmdCue` (`rb_local_data_status`);
-- index: djmd_cue_rb_local_deleted
CREATE INDEX `djmd_cue_rb_local_deleted` ON `djmdCue` (`rb_local_deleted`);
-- index: djmd_cue_rb_local_usn__i_d
CREATE INDEX `djmd_cue_rb_local_usn__i_d` ON `djmdCue` (`rb_local_usn`, `ID`);

-- djmdDevice : 接続デバイス（USBドライブ等）の登録情報
CREATE TABLE `djmdDevice` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `MasterDBID` VARCHAR(255) DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_device__u_u_i_d
CREATE INDEX `djmd_device__u_u_i_d` ON `djmdDevice` (`UUID`);
-- index: djmd_device_rb_data_status
CREATE INDEX `djmd_device_rb_data_status` ON `djmdDevice` (`rb_data_status`);
-- index: djmd_device_rb_local_data_status
CREATE INDEX `djmd_device_rb_local_data_status` ON `djmdDevice` (`rb_local_data_status`);
-- index: djmd_device_rb_local_deleted
CREATE INDEX `djmd_device_rb_local_deleted` ON `djmdDevice` (`rb_local_deleted`);
-- index: djmd_device_rb_local_usn__i_d
CREATE INDEX `djmd_device_rb_local_usn__i_d` ON `djmdDevice` (`rb_local_usn`, `ID`);

-- djmdGenre : ジャンルマスタ
CREATE TABLE `djmdGenre` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Name` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_genre__name
CREATE INDEX `djmd_genre__name` ON `djmdGenre` (`Name`);
-- index: djmd_genre__u_u_i_d
CREATE INDEX `djmd_genre__u_u_i_d` ON `djmdGenre` (`UUID`);
-- index: djmd_genre_rb_data_status
CREATE INDEX `djmd_genre_rb_data_status` ON `djmdGenre` (`rb_data_status`);
-- index: djmd_genre_rb_local_data_status
CREATE INDEX `djmd_genre_rb_local_data_status` ON `djmdGenre` (`rb_local_data_status`);
-- index: djmd_genre_rb_local_deleted
CREATE INDEX `djmd_genre_rb_local_deleted` ON `djmdGenre` (`rb_local_deleted`);
-- index: djmd_genre_rb_local_usn__i_d
CREATE INDEX `djmd_genre_rb_local_usn__i_d` ON `djmdGenre` (`rb_local_usn`, `ID`);

-- djmdHistory : 再生履歴のフォルダ（日付ごとのセッション）。ツリー構造でParentIDを持つ
CREATE TABLE `djmdHistory` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `DateCreated` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_history__name
CREATE INDEX `djmd_history__name` ON `djmdHistory` (`Name`);
-- index: djmd_history__parent_i_d
CREATE INDEX `djmd_history__parent_i_d` ON `djmdHistory` (`ParentID`);
-- index: djmd_history__u_u_i_d
CREATE INDEX `djmd_history__u_u_i_d` ON `djmdHistory` (`UUID`);
-- index: djmd_history_rb_data_status
CREATE INDEX `djmd_history_rb_data_status` ON `djmdHistory` (`rb_data_status`);
-- index: djmd_history_rb_local_data_status
CREATE INDEX `djmd_history_rb_local_data_status` ON `djmdHistory` (`rb_local_data_status`);
-- index: djmd_history_rb_local_deleted
CREATE INDEX `djmd_history_rb_local_deleted` ON `djmdHistory` (`rb_local_deleted`);
-- index: djmd_history_rb_local_usn__i_d
CREATE INDEX `djmd_history_rb_local_usn__i_d` ON `djmdHistory` (`rb_local_usn`, `ID`);

-- djmdHotCueBanklist : ホットキューバンクリストの定義。名前・画像・ツリー構造
CREATE TABLE `djmdHotCueBanklist` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `ImagePath` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_hot_cue_banklist__name
CREATE INDEX `djmd_hot_cue_banklist__name` ON `djmdHotCueBanklist` (`Name`);
-- index: djmd_hot_cue_banklist__parent_i_d
CREATE INDEX `djmd_hot_cue_banklist__parent_i_d` ON `djmdHotCueBanklist` (`ParentID`);
-- index: djmd_hot_cue_banklist__u_u_i_d
CREATE INDEX `djmd_hot_cue_banklist__u_u_i_d` ON `djmdHotCueBanklist` (`UUID`);
-- index: djmd_hot_cue_banklist_rb_data_status
CREATE INDEX `djmd_hot_cue_banklist_rb_data_status` ON `djmdHotCueBanklist` (`rb_data_status`);
-- index: djmd_hot_cue_banklist_rb_local_data_status
CREATE INDEX `djmd_hot_cue_banklist_rb_local_data_status` ON `djmdHotCueBanklist` (`rb_local_data_status`);
-- index: djmd_hot_cue_banklist_rb_local_deleted
CREATE INDEX `djmd_hot_cue_banklist_rb_local_deleted` ON `djmdHotCueBanklist` (`rb_local_deleted`);
-- index: djmd_hot_cue_banklist_rb_local_usn__i_d
CREATE INDEX `djmd_hot_cue_banklist_rb_local_usn__i_d` ON `djmdHotCueBanklist` (`rb_local_usn`, `ID`);

-- djmdKey : 音楽キー（調）マスタ。ScaleName（例: Am, Cmaj）と表示順
CREATE TABLE `djmdKey` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ScaleName` VARCHAR(255) DEFAULT NULL,
  `Seq` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_key__scale_name
CREATE INDEX `djmd_key__scale_name` ON `djmdKey` (`ScaleName`);
-- index: djmd_key__u_u_i_d
CREATE INDEX `djmd_key__u_u_i_d` ON `djmdKey` (`UUID`);
-- index: djmd_key_rb_data_status
CREATE INDEX `djmd_key_rb_data_status` ON `djmdKey` (`rb_data_status`);
-- index: djmd_key_rb_local_data_status
CREATE INDEX `djmd_key_rb_local_data_status` ON `djmdKey` (`rb_local_data_status`);
-- index: djmd_key_rb_local_deleted
CREATE INDEX `djmd_key_rb_local_deleted` ON `djmdKey` (`rb_local_deleted`);
-- index: djmd_key_rb_local_usn__i_d
CREATE INDEX `djmd_key_rb_local_usn__i_d` ON `djmdKey` (`rb_local_usn`, `ID`);

-- djmdLabel : レコードレーベルマスタ
CREATE TABLE `djmdLabel` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Name` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_label__name
CREATE INDEX `djmd_label__name` ON `djmdLabel` (`Name`);
-- index: djmd_label__u_u_i_d
CREATE INDEX `djmd_label__u_u_i_d` ON `djmdLabel` (`UUID`);
-- index: djmd_label_rb_data_status
CREATE INDEX `djmd_label_rb_data_status` ON `djmdLabel` (`rb_data_status`);
-- index: djmd_label_rb_local_data_status
CREATE INDEX `djmd_label_rb_local_data_status` ON `djmdLabel` (`rb_local_data_status`);
-- index: djmd_label_rb_local_deleted
CREATE INDEX `djmd_label_rb_local_deleted` ON `djmdLabel` (`rb_local_deleted`);
-- index: djmd_label_rb_local_usn__i_d
CREATE INDEX `djmd_label_rb_local_usn__i_d` ON `djmdLabel` (`rb_local_usn`, `ID`);

-- djmdMenuItems : UIメニュー項目の定義。カテゴリやソートで参照される
CREATE TABLE `djmdMenuItems` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Class` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_menu_items__u_u_i_d
CREATE INDEX `djmd_menu_items__u_u_i_d` ON `djmdMenuItems` (`UUID`);
-- index: djmd_menu_items_rb_data_status
CREATE INDEX `djmd_menu_items_rb_data_status` ON `djmdMenuItems` (`rb_data_status`);
-- index: djmd_menu_items_rb_local_data_status
CREATE INDEX `djmd_menu_items_rb_local_data_status` ON `djmdMenuItems` (`rb_local_data_status`);
-- index: djmd_menu_items_rb_local_deleted
CREATE INDEX `djmd_menu_items_rb_local_deleted` ON `djmdMenuItems` (`rb_local_deleted`);
-- index: djmd_menu_items_rb_local_usn__i_d
CREATE INDEX `djmd_menu_items_rb_local_usn__i_d` ON `djmdMenuItems` (`rb_local_usn`, `ID`);

-- djmdMixerParam : 楽曲ごとのミキサーパラメータ（ゲイン・ピーク値のHigh/Low）
CREATE TABLE `djmdMixerParam` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `GainHigh` INTEGER DEFAULT NULL,
  `GainLow` INTEGER DEFAULT NULL,
  `PeakHigh` INTEGER DEFAULT NULL,
  `PeakLow` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_mixer_param__content_i_d_rb_local_deleted
CREATE INDEX `djmd_mixer_param__content_i_d_rb_local_deleted` ON `djmdMixerParam` (`ContentID`, `rb_local_deleted`);
-- index: djmd_mixer_param__u_u_i_d
CREATE INDEX `djmd_mixer_param__u_u_i_d` ON `djmdMixerParam` (`UUID`);
-- index: djmd_mixer_param_rb_data_status
CREATE INDEX `djmd_mixer_param_rb_data_status` ON `djmdMixerParam` (`rb_data_status`);
-- index: djmd_mixer_param_rb_local_data_status
CREATE INDEX `djmd_mixer_param_rb_local_data_status` ON `djmdMixerParam` (`rb_local_data_status`);
-- index: djmd_mixer_param_rb_local_deleted
CREATE INDEX `djmd_mixer_param_rb_local_deleted` ON `djmdMixerParam` (`rb_local_deleted`);
-- index: djmd_mixer_param_rb_local_usn__i_d
CREATE INDEX `djmd_mixer_param_rb_local_usn__i_d` ON `djmdMixerParam` (`rb_local_usn`, `ID`);

-- djmdMyTag : ユーザー定義タグ（My Tag）のマスタ。ツリー構造
CREATE TABLE `djmdMyTag` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_my_tag__parent_i_d
CREATE INDEX `djmd_my_tag__parent_i_d` ON `djmdMyTag` (`ParentID`);
-- index: djmd_my_tag__seq
CREATE INDEX `djmd_my_tag__seq` ON `djmdMyTag` (`Seq`);
-- index: djmd_my_tag__u_u_i_d
CREATE INDEX `djmd_my_tag__u_u_i_d` ON `djmdMyTag` (`UUID`);
-- index: djmd_my_tag_rb_data_status
CREATE INDEX `djmd_my_tag_rb_data_status` ON `djmdMyTag` (`rb_data_status`);
-- index: djmd_my_tag_rb_local_data_status
CREATE INDEX `djmd_my_tag_rb_local_data_status` ON `djmdMyTag` (`rb_local_data_status`);
-- index: djmd_my_tag_rb_local_deleted
CREATE INDEX `djmd_my_tag_rb_local_deleted` ON `djmdMyTag` (`rb_local_deleted`);
-- index: djmd_my_tag_rb_local_usn__i_d
CREATE INDEX `djmd_my_tag_rb_local_usn__i_d` ON `djmdMyTag` (`rb_local_usn`, `ID`);

-- djmdPlaylist : プレイリストの定義。名前・画像・ツリー構造（フォルダ/リスト）。SmartListはスマートプレイリストの条件
CREATE TABLE `djmdPlaylist` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `ImagePath` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `SmartList` TEXT DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_playlist__attribute
CREATE INDEX `djmd_playlist__attribute` ON `djmdPlaylist` (`Attribute`);
-- index: djmd_playlist__name
CREATE INDEX `djmd_playlist__name` ON `djmdPlaylist` (`Name`);
-- index: djmd_playlist__parent_i_d
CREATE INDEX `djmd_playlist__parent_i_d` ON `djmdPlaylist` (`ParentID`);
-- index: djmd_playlist__seq
CREATE INDEX `djmd_playlist__seq` ON `djmdPlaylist` (`Seq`);
-- index: djmd_playlist__u_u_i_d
CREATE INDEX `djmd_playlist__u_u_i_d` ON `djmdPlaylist` (`UUID`);
-- index: djmd_playlist_rb_data_status
CREATE INDEX `djmd_playlist_rb_data_status` ON `djmdPlaylist` (`rb_data_status`);
-- index: djmd_playlist_rb_local_data_status
CREATE INDEX `djmd_playlist_rb_local_data_status` ON `djmdPlaylist` (`rb_local_data_status`);
-- index: djmd_playlist_rb_local_deleted
CREATE INDEX `djmd_playlist_rb_local_deleted` ON `djmdPlaylist` (`rb_local_deleted`);
-- index: djmd_playlist_rb_local_usn__i_d
CREATE INDEX `djmd_playlist_rb_local_usn__i_d` ON `djmdPlaylist` (`rb_local_usn`, `ID`);

-- djmdProperty : データベース自体のプロパティ。DBバージョン・ドライブ情報
CREATE TABLE `djmdProperty` (
  `DBID` VARCHAR(255) PRIMARY KEY,
  `DBVersion` VARCHAR(255) DEFAULT NULL,
  `BaseDBDrive` VARCHAR(255) DEFAULT NULL,
  `CurrentDBDrive` VARCHAR(255) DEFAULT NULL,
  `DeviceID` VARCHAR(255) DEFAULT NULL,
  `Reserved1` TEXT DEFAULT NULL,
  `Reserved2` TEXT DEFAULT NULL,
  `Reserved3` TEXT DEFAULT NULL,
  `Reserved4` TEXT DEFAULT NULL,
  `Reserved5` TEXT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);

-- djmdRecommendLike : 2曲間のレコメンド類似度スコア
CREATE TABLE `djmdRecommendLike` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID1` VARCHAR(255) DEFAULT NULL,
  `ContentID2` VARCHAR(255) DEFAULT NULL,
  `LikeRate` INTEGER DEFAULT NULL,
  `DataCreatedH` INTEGER DEFAULT NULL,
  `DataCreatedL` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_recommend_like__content_i_d1__content_i_d2
CREATE INDEX `djmd_recommend_like__content_i_d1__content_i_d2` ON `djmdRecommendLike` (`ContentID1`, `ContentID2`);
-- index: djmd_recommend_like__content_i_d2
CREATE INDEX `djmd_recommend_like__content_i_d2` ON `djmdRecommendLike` (`ContentID2`);
-- index: djmd_recommend_like__u_u_i_d
CREATE INDEX `djmd_recommend_like__u_u_i_d` ON `djmdRecommendLike` (`UUID`);
-- index: djmd_recommend_like_rb_data_status
CREATE INDEX `djmd_recommend_like_rb_data_status` ON `djmdRecommendLike` (`rb_data_status`);
-- index: djmd_recommend_like_rb_local_data_status
CREATE INDEX `djmd_recommend_like_rb_local_data_status` ON `djmdRecommendLike` (`rb_local_data_status`);
-- index: djmd_recommend_like_rb_local_deleted
CREATE INDEX `djmd_recommend_like_rb_local_deleted` ON `djmdRecommendLike` (`rb_local_deleted`);
-- index: djmd_recommend_like_rb_local_usn__i_d
CREATE INDEX `djmd_recommend_like_rb_local_usn__i_d` ON `djmdRecommendLike` (`rb_local_usn`, `ID`);

-- djmdRelatedTracks : 関連トラックリストの定義。名前・条件・ツリー構造
CREATE TABLE `djmdRelatedTracks` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `Criteria` TEXT DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_related_tracks__name
CREATE INDEX `djmd_related_tracks__name` ON `djmdRelatedTracks` (`Name`);
-- index: djmd_related_tracks__parent_i_d
CREATE INDEX `djmd_related_tracks__parent_i_d` ON `djmdRelatedTracks` (`ParentID`);
-- index: djmd_related_tracks__seq
CREATE INDEX `djmd_related_tracks__seq` ON `djmdRelatedTracks` (`Seq`);
-- index: djmd_related_tracks__u_u_i_d
CREATE INDEX `djmd_related_tracks__u_u_i_d` ON `djmdRelatedTracks` (`UUID`);
-- index: djmd_related_tracks_rb_data_status
CREATE INDEX `djmd_related_tracks_rb_data_status` ON `djmdRelatedTracks` (`rb_data_status`);
-- index: djmd_related_tracks_rb_local_data_status
CREATE INDEX `djmd_related_tracks_rb_local_data_status` ON `djmdRelatedTracks` (`rb_local_data_status`);
-- index: djmd_related_tracks_rb_local_deleted
CREATE INDEX `djmd_related_tracks_rb_local_deleted` ON `djmdRelatedTracks` (`rb_local_deleted`);
-- index: djmd_related_tracks_rb_local_usn__i_d
CREATE INDEX `djmd_related_tracks_rb_local_usn__i_d` ON `djmdRelatedTracks` (`rb_local_usn`, `ID`);

-- djmdSampler : サンプラーリストの定義。ツリー構造
CREATE TABLE `djmdSampler` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Seq` INTEGER DEFAULT NULL,
  `Name` VARCHAR(255) DEFAULT NULL,
  `Attribute` INTEGER DEFAULT NULL,
  `ParentID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_sampler__name
CREATE INDEX `djmd_sampler__name` ON `djmdSampler` (`Name`);
-- index: djmd_sampler__parent_i_d
CREATE INDEX `djmd_sampler__parent_i_d` ON `djmdSampler` (`ParentID`);
-- index: djmd_sampler__seq
CREATE INDEX `djmd_sampler__seq` ON `djmdSampler` (`Seq`);
-- index: djmd_sampler__u_u_i_d
CREATE INDEX `djmd_sampler__u_u_i_d` ON `djmdSampler` (`UUID`);
-- index: djmd_sampler_rb_data_status
CREATE INDEX `djmd_sampler_rb_data_status` ON `djmdSampler` (`rb_data_status`);
-- index: djmd_sampler_rb_local_data_status
CREATE INDEX `djmd_sampler_rb_local_data_status` ON `djmdSampler` (`rb_local_data_status`);
-- index: djmd_sampler_rb_local_deleted
CREATE INDEX `djmd_sampler_rb_local_deleted` ON `djmdSampler` (`rb_local_deleted`);
-- index: djmd_sampler_rb_local_usn__i_d
CREATE INDEX `djmd_sampler_rb_local_usn__i_d` ON `djmdSampler` (`rb_local_usn`, `ID`);

-- djmdSongHistory : 再生履歴の個別エントリ。どのHistory（セッション）にどのContentが何番目か
CREATE TABLE `djmdSongHistory` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `HistoryID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_history__content_i_d_rb_local_deleted
CREATE INDEX `djmd_song_history__content_i_d_rb_local_deleted` ON `djmdSongHistory` (`ContentID`, `rb_local_deleted`);
-- index: djmd_song_history__history_i_d
CREATE INDEX `djmd_song_history__history_i_d` ON `djmdSongHistory` (`HistoryID`);
-- index: djmd_song_history__u_u_i_d
CREATE INDEX `djmd_song_history__u_u_i_d` ON `djmdSongHistory` (`UUID`);
-- index: djmd_song_history_rb_data_status
CREATE INDEX `djmd_song_history_rb_data_status` ON `djmdSongHistory` (`rb_data_status`);
-- index: djmd_song_history_rb_local_data_status
CREATE INDEX `djmd_song_history_rb_local_data_status` ON `djmdSongHistory` (`rb_local_data_status`);
-- index: djmd_song_history_rb_local_deleted
CREATE INDEX `djmd_song_history_rb_local_deleted` ON `djmdSongHistory` (`rb_local_deleted`);
-- index: djmd_song_history_rb_local_usn__i_d
CREATE INDEX `djmd_song_history_rb_local_usn__i_d` ON `djmdSongHistory` (`rb_local_usn`, `ID`);

-- djmdSongHotCueBanklist : ホットキューバンクリストの個別エントリ。バンクリスト内の各キュー位置・色・ループ情報
CREATE TABLE `djmdSongHotCueBanklist` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `HotCueBanklistID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `CueID` VARCHAR(255) DEFAULT NULL,
  `InMsec` INTEGER DEFAULT NULL,
  `InFrame` INTEGER DEFAULT NULL,
  `InMpegFrame` INTEGER DEFAULT NULL,
  `InMpegAbs` INTEGER DEFAULT NULL,
  `OutMsec` INTEGER DEFAULT NULL,
  `OutFrame` INTEGER DEFAULT NULL,
  `OutMpegFrame` INTEGER DEFAULT NULL,
  `OutMpegAbs` INTEGER DEFAULT NULL,
  `Color` INTEGER DEFAULT NULL,
  `ColorTableIndex` INTEGER DEFAULT NULL,
  `ActiveLoop` INTEGER DEFAULT NULL,
  `Comment` VARCHAR(255) DEFAULT NULL,
  `BeatLoopSize` INTEGER DEFAULT NULL,
  `CueMicrosec` INTEGER DEFAULT NULL,
  `InPointSeekInfo` VARCHAR(255) DEFAULT NULL,
  `OutPointSeekInfo` VARCHAR(255) DEFAULT NULL,
  `HotCueBanklistUUID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_hot_cue_banklist__content_i_d
CREATE INDEX `djmd_song_hot_cue_banklist__content_i_d` ON `djmdSongHotCueBanklist` (`ContentID`);
-- index: djmd_song_hot_cue_banklist__hot_cue_banklist_i_d
CREATE INDEX `djmd_song_hot_cue_banklist__hot_cue_banklist_i_d` ON `djmdSongHotCueBanklist` (`HotCueBanklistID`);
-- index: djmd_song_hot_cue_banklist__hot_cue_banklist_u_u_i_d
CREATE INDEX `djmd_song_hot_cue_banklist__hot_cue_banklist_u_u_i_d` ON `djmdSongHotCueBanklist` (`HotCueBanklistUUID`);
-- index: djmd_song_hot_cue_banklist__u_u_i_d
CREATE INDEX `djmd_song_hot_cue_banklist__u_u_i_d` ON `djmdSongHotCueBanklist` (`UUID`);
-- index: djmd_song_hot_cue_banklist_rb_data_status
CREATE INDEX `djmd_song_hot_cue_banklist_rb_data_status` ON `djmdSongHotCueBanklist` (`rb_data_status`);
-- index: djmd_song_hot_cue_banklist_rb_local_data_status
CREATE INDEX `djmd_song_hot_cue_banklist_rb_local_data_status` ON `djmdSongHotCueBanklist` (`rb_local_data_status`);
-- index: djmd_song_hot_cue_banklist_rb_local_deleted
CREATE INDEX `djmd_song_hot_cue_banklist_rb_local_deleted` ON `djmdSongHotCueBanklist` (`rb_local_deleted`);
-- index: djmd_song_hot_cue_banklist_rb_local_usn__i_d
CREATE INDEX `djmd_song_hot_cue_banklist_rb_local_usn__i_d` ON `djmdSongHotCueBanklist` (`rb_local_usn`, `ID`);

-- djmdSongMyTag : 楽曲とMyTagの紐付け（中間テーブル）
CREATE TABLE `djmdSongMyTag` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `MyTagID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_my_tag__content_i_d_rb_local_deleted
CREATE INDEX `djmd_song_my_tag__content_i_d_rb_local_deleted` ON `djmdSongMyTag` (`ContentID`, `rb_local_deleted`);
-- index: djmd_song_my_tag__my_tag_i_d
CREATE INDEX `djmd_song_my_tag__my_tag_i_d` ON `djmdSongMyTag` (`MyTagID`);
-- index: djmd_song_my_tag__my_tag_i_d_rb_local_deleted__i_d
CREATE INDEX `djmd_song_my_tag__my_tag_i_d_rb_local_deleted__i_d` ON `djmdSongMyTag` (`MyTagID`, `rb_local_deleted`, `ID`);
-- index: djmd_song_my_tag__u_u_i_d
CREATE INDEX `djmd_song_my_tag__u_u_i_d` ON `djmdSongMyTag` (`UUID`);
-- index: djmd_song_my_tag_rb_data_status
CREATE INDEX `djmd_song_my_tag_rb_data_status` ON `djmdSongMyTag` (`rb_data_status`);
-- index: djmd_song_my_tag_rb_local_data_status
CREATE INDEX `djmd_song_my_tag_rb_local_data_status` ON `djmdSongMyTag` (`rb_local_data_status`);
-- index: djmd_song_my_tag_rb_local_deleted
CREATE INDEX `djmd_song_my_tag_rb_local_deleted` ON `djmdSongMyTag` (`rb_local_deleted`);
-- index: djmd_song_my_tag_rb_local_usn__i_d
CREATE INDEX `djmd_song_my_tag_rb_local_usn__i_d` ON `djmdSongMyTag` (`rb_local_usn`, `ID`);

-- djmdSongPlaylist : プレイリストの個別エントリ。どのPlaylistにどのContentが何番目か
CREATE TABLE `djmdSongPlaylist` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `PlaylistID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_playlist__content_i_d_rb_local_deleted
CREATE INDEX `djmd_song_playlist__content_i_d_rb_local_deleted` ON `djmdSongPlaylist` (`ContentID`, `rb_local_deleted`);
-- index: djmd_song_playlist__playlist_i_d
CREATE INDEX `djmd_song_playlist__playlist_i_d` ON `djmdSongPlaylist` (`PlaylistID`);
-- index: djmd_song_playlist__playlist_i_d__i_d
CREATE INDEX `djmd_song_playlist__playlist_i_d__i_d` ON `djmdSongPlaylist` (`PlaylistID`, `ID`);
-- index: djmd_song_playlist__u_u_i_d
CREATE INDEX `djmd_song_playlist__u_u_i_d` ON `djmdSongPlaylist` (`UUID`);
-- index: djmd_song_playlist_rb_data_status
CREATE INDEX `djmd_song_playlist_rb_data_status` ON `djmdSongPlaylist` (`rb_data_status`);
-- index: djmd_song_playlist_rb_local_data_status
CREATE INDEX `djmd_song_playlist_rb_local_data_status` ON `djmdSongPlaylist` (`rb_local_data_status`);
-- index: djmd_song_playlist_rb_local_deleted
CREATE INDEX `djmd_song_playlist_rb_local_deleted` ON `djmdSongPlaylist` (`rb_local_deleted`);
-- index: djmd_song_playlist_rb_local_usn__i_d
CREATE INDEX `djmd_song_playlist_rb_local_usn__i_d` ON `djmdSongPlaylist` (`rb_local_usn`, `ID`);

-- djmdSongRelatedTracks : 関連トラックリストの個別エントリ
CREATE TABLE `djmdSongRelatedTracks` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `RelatedTracksID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_related_tracks__content_i_d
CREATE INDEX `djmd_song_related_tracks__content_i_d` ON `djmdSongRelatedTracks` (`ContentID`);
-- index: djmd_song_related_tracks__related_tracks_i_d
CREATE INDEX `djmd_song_related_tracks__related_tracks_i_d` ON `djmdSongRelatedTracks` (`RelatedTracksID`);
-- index: djmd_song_related_tracks__u_u_i_d
CREATE INDEX `djmd_song_related_tracks__u_u_i_d` ON `djmdSongRelatedTracks` (`UUID`);
-- index: djmd_song_related_tracks_rb_data_status
CREATE INDEX `djmd_song_related_tracks_rb_data_status` ON `djmdSongRelatedTracks` (`rb_data_status`);
-- index: djmd_song_related_tracks_rb_local_data_status
CREATE INDEX `djmd_song_related_tracks_rb_local_data_status` ON `djmdSongRelatedTracks` (`rb_local_data_status`);
-- index: djmd_song_related_tracks_rb_local_deleted
CREATE INDEX `djmd_song_related_tracks_rb_local_deleted` ON `djmdSongRelatedTracks` (`rb_local_deleted`);
-- index: djmd_song_related_tracks_rb_local_usn__i_d
CREATE INDEX `djmd_song_related_tracks_rb_local_usn__i_d` ON `djmdSongRelatedTracks` (`rb_local_usn`, `ID`);

-- djmdSongSampler : サンプラーリストの個別エントリ
CREATE TABLE `djmdSongSampler` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `SamplerID` VARCHAR(255) DEFAULT NULL,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_sampler__content_i_d
CREATE INDEX `djmd_song_sampler__content_i_d` ON `djmdSongSampler` (`ContentID`);
-- index: djmd_song_sampler__sampler_i_d
CREATE INDEX `djmd_song_sampler__sampler_i_d` ON `djmdSongSampler` (`SamplerID`);
-- index: djmd_song_sampler__u_u_i_d
CREATE INDEX `djmd_song_sampler__u_u_i_d` ON `djmdSongSampler` (`UUID`);
-- index: djmd_song_sampler_rb_data_status
CREATE INDEX `djmd_song_sampler_rb_data_status` ON `djmdSongSampler` (`rb_data_status`);
-- index: djmd_song_sampler_rb_local_data_status
CREATE INDEX `djmd_song_sampler_rb_local_data_status` ON `djmdSongSampler` (`rb_local_data_status`);
-- index: djmd_song_sampler_rb_local_deleted
CREATE INDEX `djmd_song_sampler_rb_local_deleted` ON `djmdSongSampler` (`rb_local_deleted`);
-- index: djmd_song_sampler_rb_local_usn__i_d
CREATE INDEX `djmd_song_sampler_rb_local_usn__i_d` ON `djmdSongSampler` (`rb_local_usn`, `ID`);

-- djmdSongTagList : 楽曲のタグリスト紐付け
CREATE TABLE `djmdSongTagList` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `ContentID` VARCHAR(255) DEFAULT NULL,
  `TrackNo` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_song_tag_list__content_i_d
CREATE INDEX `djmd_song_tag_list__content_i_d` ON `djmdSongTagList` (`ContentID`);
-- index: djmd_song_tag_list__u_u_i_d
CREATE INDEX `djmd_song_tag_list__u_u_i_d` ON `djmdSongTagList` (`UUID`);
-- index: djmd_song_tag_list_rb_data_status
CREATE INDEX `djmd_song_tag_list_rb_data_status` ON `djmdSongTagList` (`rb_data_status`);
-- index: djmd_song_tag_list_rb_local_data_status
CREATE INDEX `djmd_song_tag_list_rb_local_data_status` ON `djmdSongTagList` (`rb_local_data_status`);
-- index: djmd_song_tag_list_rb_local_deleted
CREATE INDEX `djmd_song_tag_list_rb_local_deleted` ON `djmdSongTagList` (`rb_local_deleted`);
-- index: djmd_song_tag_list_rb_local_usn__i_d
CREATE INDEX `djmd_song_tag_list_rb_local_usn__i_d` ON `djmdSongTagList` (`rb_local_usn`, `ID`);

-- djmdSort : ブラウズ画面のソート項目の表示順・有効無効
CREATE TABLE `djmdSort` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `MenuItemID` VARCHAR(255) DEFAULT NULL,
  `Seq` INTEGER DEFAULT NULL,
  `Disable` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: djmd_sort__u_u_i_d
CREATE INDEX `djmd_sort__u_u_i_d` ON `djmdSort` (`UUID`);
-- index: djmd_sort_rb_data_status
CREATE INDEX `djmd_sort_rb_data_status` ON `djmdSort` (`rb_data_status`);
-- index: djmd_sort_rb_local_data_status
CREATE INDEX `djmd_sort_rb_local_data_status` ON `djmdSort` (`rb_local_data_status`);
-- index: djmd_sort_rb_local_deleted
CREATE INDEX `djmd_sort_rb_local_deleted` ON `djmdSort` (`rb_local_deleted`);
-- index: djmd_sort_rb_local_usn__i_d
CREATE INDEX `djmd_sort_rb_local_usn__i_d` ON `djmdSort` (`rb_local_usn`, `ID`);

-- hotCueBanklistCue : ホットキューバンクリストのキュー情報をまとめてJSON的に保持
CREATE TABLE `hotCueBanklistCue` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `HotCueBanklistID` VARCHAR(255) DEFAULT NULL,
  `Cues` TEXT DEFAULT NULL,
  `rb_cue_count` INTEGER DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: hot_cue_banklist_cue__hot_cue_banklist_i_d
CREATE INDEX `hot_cue_banklist_cue__hot_cue_banklist_i_d` ON `hotCueBanklistCue` (`HotCueBanklistID`);
-- index: hot_cue_banklist_cue__u_u_i_d
CREATE INDEX `hot_cue_banklist_cue__u_u_i_d` ON `hotCueBanklistCue` (`UUID`);
-- index: hot_cue_banklist_cue_rb_cue_count
CREATE INDEX `hot_cue_banklist_cue_rb_cue_count` ON `hotCueBanklistCue` (`rb_cue_count`);
-- index: hot_cue_banklist_cue_rb_data_status
CREATE INDEX `hot_cue_banklist_cue_rb_data_status` ON `hotCueBanklistCue` (`rb_data_status`);
-- index: hot_cue_banklist_cue_rb_local_data_status
CREATE INDEX `hot_cue_banklist_cue_rb_local_data_status` ON `hotCueBanklistCue` (`rb_local_data_status`);
-- index: hot_cue_banklist_cue_rb_local_deleted
CREATE INDEX `hot_cue_banklist_cue_rb_local_deleted` ON `hotCueBanklistCue` (`rb_local_deleted`);
-- index: hot_cue_banklist_cue_rb_local_usn__i_d
CREATE INDEX `hot_cue_banklist_cue_rb_local_usn__i_d` ON `hotCueBanklistCue` (`rb_local_usn`, `ID`);

-- imageFile : 画像ファイル（アルバムアート等）の実体管理。クラウド同期のパス・ハッシュ・進捗
CREATE TABLE `imageFile` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `TableName` VARCHAR(255) DEFAULT NULL,
  `TargetUUID` VARCHAR(255) DEFAULT NULL,
  `TargetID` VARCHAR(255) DEFAULT NULL,
  `Path` VARCHAR(255) DEFAULT NULL,
  `Hash` VARCHAR(255) DEFAULT NULL,
  `Size` INTEGER DEFAULT NULL,
  `rb_local_path` VARCHAR(255) DEFAULT NULL,
  `rb_insync_hash` VARCHAR(255) DEFAULT NULL,
  `rb_insync_local_usn` BIGINT DEFAULT NULL,
  `rb_file_hash_dirty` INTEGER DEFAULT 0,
  `rb_local_file_status` INTEGER DEFAULT 0,
  `rb_in_progress` TINYINT(1) DEFAULT 0,
  `rb_process_type` INTEGER DEFAULT 0,
  `rb_temp_path` VARCHAR(255) DEFAULT NULL,
  `rb_priority` INTEGER DEFAULT 50,
  `rb_file_size_dirty` INTEGER DEFAULT 0,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: image_file__table_name__target_i_d
CREATE INDEX `image_file__table_name__target_i_d` ON `imageFile` (`TableName`, `TargetID`);
-- index: image_file__table_name__target_u_u_i_d
CREATE INDEX `image_file__table_name__target_u_u_i_d` ON `imageFile` (`TableName`, `TargetUUID`);
-- index: image_file__table_name__target_u_u_i_d__i_d
CREATE INDEX `image_file__table_name__target_u_u_i_d__i_d` ON `imageFile` (`TableName`, `TargetUUID`, `ID`);
-- index: image_file__u_u_i_d
CREATE INDEX `image_file__u_u_i_d` ON `imageFile` (`UUID`);
-- index: image_file_rb_data_status
CREATE INDEX `image_file_rb_data_status` ON `imageFile` (`rb_data_status`);
-- index: image_file_rb_file_hash_dirty
CREATE INDEX `image_file_rb_file_hash_dirty` ON `imageFile` (`rb_file_hash_dirty`);
-- index: image_file_rb_file_size_dirty
CREATE INDEX `image_file_rb_file_size_dirty` ON `imageFile` (`rb_file_size_dirty`);
-- index: image_file_rb_local_data_status
CREATE INDEX `image_file_rb_local_data_status` ON `imageFile` (`rb_local_data_status`);
-- index: image_file_rb_local_deleted
CREATE INDEX `image_file_rb_local_deleted` ON `imageFile` (`rb_local_deleted`);
-- index: image_file_rb_local_deleted_rb_in_progress_rb_local_file_status_rb_process_type_rb_priority
CREATE INDEX `image_file_rb_local_deleted_rb_in_progress_rb_local_file_status_rb_process_type_rb_priority` ON `imageFile` (`rb_local_deleted`, `rb_in_progress`, `rb_local_file_status`, `rb_process_type`, `rb_priority`);
-- index: image_file_rb_local_usn__i_d
CREATE INDEX `image_file_rb_local_usn__i_d` ON `imageFile` (`rb_local_usn`, `ID`);

-- settingFile : 設定ファイルの同期管理。パス・ハッシュで変更検知
CREATE TABLE `settingFile` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `Path` VARCHAR(255) DEFAULT NULL,
  `Hash` VARCHAR(255) DEFAULT NULL,
  `Size` INTEGER DEFAULT NULL,
  `rb_local_path` VARCHAR(255) DEFAULT NULL,
  `rb_insync_hash` VARCHAR(255) DEFAULT NULL,
  `rb_insync_local_usn` BIGINT DEFAULT NULL,
  `rb_file_hash_dirty` INTEGER DEFAULT 0,
  `rb_file_size_dirty` INTEGER DEFAULT 0,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: setting_file__u_u_i_d
CREATE INDEX `setting_file__u_u_i_d` ON `settingFile` (`UUID`);
-- index: setting_file_rb_data_status
CREATE INDEX `setting_file_rb_data_status` ON `settingFile` (`rb_data_status`);
-- index: setting_file_rb_file_hash_dirty
CREATE INDEX `setting_file_rb_file_hash_dirty` ON `settingFile` (`rb_file_hash_dirty`);
-- index: setting_file_rb_file_size_dirty
CREATE INDEX `setting_file_rb_file_size_dirty` ON `settingFile` (`rb_file_size_dirty`);
-- index: setting_file_rb_local_data_status
CREATE INDEX `setting_file_rb_local_data_status` ON `settingFile` (`rb_local_data_status`);
-- index: setting_file_rb_local_deleted
CREATE INDEX `setting_file_rb_local_deleted` ON `settingFile` (`rb_local_deleted`);
-- index: setting_file_rb_local_usn__i_d
CREATE INDEX `setting_file_rb_local_usn__i_d` ON `settingFile` (`rb_local_usn`, `ID`);

-- sqlite_sequence : SQLite内部テーブル。AUTOINCREMENTの現在値を追跡
CREATE TABLE sqlite_sequence(
  name,
  seq
);

-- uuidIDMap : UUID と内部ID のマッピング。テーブル横断でUUIDから実IDを引く
CREATE TABLE `uuidIDMap` (
  `ID` VARCHAR(255) PRIMARY KEY,
  `TableName` VARCHAR(255) DEFAULT NULL,
  `TargetUUID` VARCHAR(255) DEFAULT NULL,
  `CurrentID` VARCHAR(255) DEFAULT NULL,
  `UUID` VARCHAR(255) DEFAULT NULL,
  `rb_data_status` INTEGER DEFAULT 0,
  `rb_local_data_status` INTEGER DEFAULT 0,
  `rb_local_deleted` TINYINT(1) DEFAULT 0,
  `rb_local_synced` TINYINT(1) DEFAULT 0,
  `usn` BIGINT DEFAULT NULL,
  `rb_local_usn` BIGINT DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL
);
-- index: uuid_i_d_map__u_u_i_d
CREATE INDEX `uuid_i_d_map__u_u_i_d` ON `uuidIDMap` (`UUID`);
-- index: uuid_i_d_map_rb_data_status
CREATE INDEX `uuid_i_d_map_rb_data_status` ON `uuidIDMap` (`rb_data_status`);
-- index: uuid_i_d_map_rb_local_data_status
CREATE INDEX `uuid_i_d_map_rb_local_data_status` ON `uuidIDMap` (`rb_local_data_status`);
-- index: uuid_i_d_map_rb_local_deleted
CREATE INDEX `uuid_i_d_map_rb_local_deleted` ON `uuidIDMap` (`rb_local_deleted`);
-- index: uuid_i_d_map_rb_local_usn__i_d
CREATE INDEX `uuid_i_d_map_rb_local_usn__i_d` ON `uuidIDMap` (`rb_local_usn`, `ID`);

-- 42 テーブル, 264 インデックス
