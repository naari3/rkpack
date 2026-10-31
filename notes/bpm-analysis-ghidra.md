# rekordbox 7.2.3 の BPM / ビートグリッド解析 (静的解析)

rekordbox 7.2.3 (Windows) が曲の BPM とビートグリッドをどう解析しているかを、Ghidra と付属ファイルの読み取りで調べた記録。
時系列で追記する。結論が覆った場合も古い項目は消さず「訂正」を追記する。観察と仮説は分けて書く。

- 対象: `C:\Program Files\rekordbox\rekordbox 7.2.3\rekordbox.exe` (image base 0x140000000)。アドレスはすべて VA。
- Ghidra プロジェクトは `notes/id-allocation-ghidra.md` と同じ rb723 (scratchpad、`/rekordbox.exe`)。
- PDB は無い。関数名は推定で付けた。`__FUNCTION__` 文字列 (例 `analyzer::AnalyzeJobBeatDetector::highPrecisionBeatAnalyze`) と RTTI 名がある所はそれに合わせた。

## 2026-10-09 調査 1

### 観察: どのバイナリか

- ANLZ タグ (PQTZ / PQT2 / PWAV / PSSI) や BPM 系の文字列・クラスは rekordbox.exe 本体にある。clatc.dll / Demucs3.dll / Spleeter.dll / rbHttpServer.exe には見当たらない。
- 文字列 `BALib.dll` (0x143862048) はあるが、インストール先に BALib.dll は無い。解析ロジックは exe 内に静的リンクされている (RTTI `BeatAnalyzer_1_0::*`, `BeatAnalyzer_2_0::*`)。
- 関係するクラス (RTTI):
  - `analyzer::AnalyzeJobBeatDetector`, `analyzer::BALibWrapper`, `analyzer::CBeatAnalyzeLib`, `analyzer::CBeatAnalyzeThread`, `analyzer::BeatAnalyzeMode`
  - `BeatAnalyzer_1_0::Analyzer` / `BeatAnalyzer_2_0::Analyzer`。下位に `peakss::BpmCountTbl`, `peakss::BeatSensorSignal`, `peakss::BeatListSignal`, `PreChecker`, `SimpleEnvelopeMaker`, `UnitEnvelopeMaker`, `UnitBeatAdjust`, `LoudViewMaker`, `ZoomViewMaker`, `IirPassFilter`, `balib::IirFilter`, `HighPathFilter1D/2D`, `LowPathFilter2D` がある。
  - `TFWrapper::Model` (tensorflow_cc.dll から import)。NumCpp (`Thirdparty\numcpp\NumCpp\...`) も静的リンクされている。
- `models\detect_beat\model7\` と `model8\` に TensorFlow (Keras 2.12) の SavedModel がある。exe に文字列 `models/detect_beat/model7/` と `.../model8/` (0x143897960 付近) がある。

### 観察: 学習済みモデル (models/detect_beat/model7, model8)

`keras_metadata.pb` の JSON と `variables.index` (BundleEntryProto) を読んだ。2 つのモデルは構造が同じで、重みファイルの大きさも同じ (423,654 byte)。

- 入力: `fltp` という名前の float32 `[batch, samples]` (生波形)。exe 側の名前は `serving_default_fltp:0`、出力は `StatefulPartitionedCall:1`。
- 前処理層:
  - `STFTLayer` が 3 つ。frame_length 1024 / 2048 / 4096、frame_step はすべて 441 (44.1kHz で 100 fps)。
  - `FilteredSpectrogram` が 3 つ。
- 本体: LSTM セルが 6 つ。重みの形は次のとおり。
  - 入力層: kernel [314,100]、recurrent [25,100]、bias [100]、[25] が 3 つ
  - 2, 3 層目: kernel [50,100] ほか同様
  - 3 つ目の [25] はピープホール結合と思われる
  - 出力層: Dense [50,3] + [3]
- 仮説: madmom の `RNNDownBeatProcessor` (Böck らの downbeat 用 BLSTM) とほぼ同じ構成と思われる。3 解像度の STFT、hop 441、対数フィルタ済みスペクトログラムとその正の差分で 314 次元、25 ユニットの双方向 LSTM が 3 層 (前向き/後ろ向きで 6 セル)、3 クラス出力 (非ビート / ビート / ダウンビート) という点が一致する。

### 観察: 呼び出しの流れ (解析ジョブ)

1. `CBeatAnalyzeThread_run_guess` (0x1410609e0、巨大な関数)
   - デコードしながら旧来の DSP 解析器 (`BALibWrapper`) を動かすと思われる。
   - 設定の解析モード (`[設定+0x2ca8]`) が 2〜4 なら `BALibWrapper` を版 2 (`BeatAnalyzer_2_0`) にし (0x1410268f0, edx=2)、それ以外は版 1 のまま。
   - `BALibWrapper_createAnalyzer` (0x141026910) を呼ぶ。44.1kHz と 48kHz で生成関数が分かれ、それ以外のサンプルレートは 48k 用に回すように見える。
2. `AnalyzeJobBeatDetector_Detect` (0x1410ded50)
   - `[param+0x18]+0x60` (解析モード) が 2, 3, 4 なら `AnalyzeJobBeatDetector_highPrecisionBeatAnalyze` (0x1410dee80) を呼ぶ。
   - それ以外は `AnalyzeJobBeatDetector_legacyBeatAnalyze` (0x1410dfbc0) を呼ぶ。
3. `highPrecisionBeatAnalyze` (0x1410dee80)
   - 旧解析器の結果から BPM (0x141026c80。版 2 なら `(int)(bpm_float*100+0.5)`)、ビート数 (0x141026cb0)、開始位置 (0x141026d30 → 0x1423e33f0) を取る。
   - ダイナミック フラグを決める。モード 2: 旧解析器の `+0x2d` フラグ (0x141026d80)。モード 3: false。モード 4: true。決めた値を 0x141026d50 で旧解析器にも設定する。
   - `hpBeat_decodeRunModelBuildJson` (0x1410e4590) を呼び、JSON (`beatList[]` に `bpm`/`pos`/`numBar`、ほかに `numChannels`, `sampleRate`, `decodeSampleRate`, `bitRate`, `bitDepth`, `decodeSamples`, `bpm`) を受け取る。
   - beatList を 12 byte 構造体 (`pos`, `numBar`, `bpm`) の配列に詰め直し、0x141026da0 で旧解析器に戻す。
   - 旧解析器から改めて BPM (0x141026e00)、ビート数 (0x141026e20)、ビート列 (0x141026e40) を取る。
   - `beatGrid_applyBpmRangeAndToPqtz` (0x1410e0080) で PQTZ 形式のビート列 (`param+0x58`) を作る。
4. `legacyBeatAnalyze` (0x1410dfbc0): 旧解析器の BPM とビート列をそのまま取り出し、同じ 0x1410e0080 を通す。
5. ANLZ の読み書き:
   - 0x14110c6a0 は `PMAI` → `PPTH` → `PVBR` → `PQTZ` の順にタグを辿って PQTZ のエントリ (u16, u16, u32 のビッグエンディアン) を読む。
   - 0x14110fb30 は PQTZ ヘッダの u16 (+0x12) を書き換える。
   - PQTZ 本体を書き出す関数は今回は特定していない (候補: 0x14110df00, 0x141114420 など、'PQTZ' を即値で持つ関数)。

### 観察: 高精度解析の本体 `hpBeat_decodeRunModelBuildJson` (0x1410e4590)

- デコーダ (0x140f45460) で 0x200 サンプルずつ L/R の float に読み込む。モノラルなら L を R にも複製する。
- 解析範囲:
  - ダイナミックでないとき: 長さ `sr*60` (60 秒) だけ。開始位置は引数 param_5 (= 旧解析器の 0x1423e33f0 の戻り値)。曲末にはみ出すときは曲末から 60 秒に寄せる。
  - ダイナミックのときは曲全体。
- `hpBeat_selectNormalOrDynamic` (0x1410e43f0):
  - 通常は `hpBeat_runTfModelNormal` (0x1410fc680) を、モデルパスの 2 本目 (`param_1[6]+0x20`) で呼ぶ。
  - ダイナミックは `hpBeat_runTfModelDynamic` (0x1410f9cd0) を、1 本目 (`param_1[6]+0`) で呼ぶ。
- `hpBeat_runTfModelNormal` (0x1410fc680):
  - `TFWrapper::Model(path, false, GetNativeSystemInfo().dwNumberOfProcessors, true)` を作る。
  - `mono = (L + R) * 0.5` を `[1, n]` のテンソルにして実行する。
  - 出力の 2 次元配列 (frames × クラス) を `dbnDownBeatTracking_numcpp` (0x1410e82c0) に渡す。
- `dbnDownBeatTracking_numcpp` (0x1410e82c0) の引数 (呼び出し元 0x1410fcf91〜0x1410fcff5 の即値):
  - beats_per_bar = {4} (配列、要素 1 個)
  - min_bpm = 70.0 (0x14532e108), max_bpm = 180.0 (0x14532e188)
  - num_tempi = 60 (0x3c), transition_lambda = 100 (0x64), observation_lambda = 16 (0x10)
  - threshold = 0.05f (0x14532d53c), correct = true, fps = 100.0 (0x14532e130)
  - 内部で `param_11 * 60.0` (fps*60) を使う。テンポごとの状態空間を作り、Viterbi らしき処理を NumCpp で行う。
  - 仮説: madmom の `DBNDownBeatTrackingProcessor` (BarStateSpace + BarTransitionModel + RNNDownBeatTrackingObservationModel + Viterbi) を C++/NumCpp に移植したものと思われる。パラメータ名と既定値 (num_tempi=60, transition_lambda=100, observation_lambda=16, threshold=0.05, correct=True, fps=100) が madmom と一致する。min/max_bpm だけが madmom の既定 (55/205) と違い 70/180。
- ダイナミック版 0x1410f9cd0:
  - 同じ DBN を同じ定数 (70/180, 60, 100, 16, 0.05, 100fps) で 3 か所から呼ぶ。
  - `sr*60`、`sr*30`、60/44100、1/44100 の定数が出てくる。
  - 仮説: 60 秒の窓を (30 秒ずつずらすなどして) 区切ってモデル + DBN を回し、結果をつなぐと思われる。詳細は未確認。
- ビートの後処理 (0x1410e4590 の中):
  - デコードのサンプルレートが 44100 でなければ、ビート時刻を `t * 44100 / sr` で補正する。
  - ダイナミックのときは 60 秒境界ごとに、前後のビート間隔の比で欠けたビートを補ったり余分なビートを消したりする。
    - 比が 1.35〜3.5 で、次の比が 0.25〜0.75 なら、間に round(比)-1 個のビートを線形補間で足す。
    - 比 < 0.3 なら消す。
    - 0.9〜1.1 の条件もある。
  - その後、全区間で「前後の比がどちらも 1.8 を超える所」に補間ビートを足す。
- `hpBeat_buildBeatListJson` (0x1410e5ee0、beats_per_bar=4):
  - 検出した最初のビートより前は、最初の間隔で 0 秒まで逆向きに延長する。
  - 各ビートに次の値を入れる。
    - `msec` = round(t*1000)
    - `pos` = (int)(sr * msec * 0.001)
    - 1 つ前のビートの `bpm` = (int)(60000 / (msec - 前の msec) * 100) (BPM×100)
    - `numBar` = 小節内の拍番号
  - 最後に検出したビートより後は、最後の間隔で曲の長さまで延長し、`numBar` を `% 4 + 1` で回す。
- 曲全体の `bpm`: `hpBeat_bpmFromBeatList_medianOrMean` (0x1410e6e20, param_2=1)。各ビートの `bpm` を集め、`"linear"` 補間で 50 パーセンタイル (= 中央値) を取る (0x1410f17c0 に 50.0 と "linear" を渡す)。param_2=0 なら平均になる。

### 観察: 旧来の DSP 解析器 `BeatAnalyzer_2_0::Analyzer`

- `BeatAnalyzer_2_0_Analyzer_init` (0x1423c62c0) の内容:
  - サンプルレートは 44100 か 48000 のみ受け付ける。
  - MXCSR に 0x8000 (FTZ) を立てる。
  - 1 フレームを `sr / 75` サンプル (44.1kHz で 588 サンプル、75 fps) にしている。
  - 下位モジュール (+0xe8, +0x590, +0x5d8, +0x640, +0x6f0, +0x3a10。最後のものは引数 150) を初期化する。
- 0x1423e33f0 (高精度解析の開始位置):
  - ビートごとの音量 (0x1423dfe30) を最大値で正規化し、差分としきい値で「曲が本格的に始まる位置」を探す。
  - 戻り値は `(60/bpm) * (そのビート番号 - 4) * sr`。始まりの 4 拍前から 60 秒を高精度解析に回すと思われる。
- 旧解析器の BPM 推定そのもの (BpmCountTbl / BeatSensorSignal 周り) は今回は読んでいない。

### 観察: BPM 範囲設定

- UI の文字列 (0x37364xx 付近): `70-180`, `58-115`, `68-135`, `78-155`, `88-175`, `98-195`, `108-215`, `118-235`, `128-255`。設定キーは `trackAnalysisBpmRange` で、既定値は 1 (0x1437cea0c)。
- 0x143433bc0 からの int ペア表を `[設定+0x6c]` (1〜10 の範囲外なら 1 に直す) で引く。表の内容は次のとおり。
  - idx0 (70,180), idx1 (70,180), idx2 (48,95), idx3 (58,115), idx4 (68,135), idx5 (78,155), idx6 (88,175), idx7 (98,195), idx8 (108,215), idx9 (118,235), idx10 (128,255)
  - 同じ表が 0x14342b180 付近にもあり、0x14104b910 などから参照されている。
- `beatGrid_applyBpmRangeAndToPqtz` (0x1410e0080):
  - ダイナミック フラグ (`[param+0x28]+0x28`) が false のときだけ範囲を適用する。
  - BPM×100 が min×100 未満なら、0x1410e0800 でビートを倍に (間を補う) して BPM を 2 倍にする。これを範囲に入るまで繰り返す。
  - max×100 を超えるなら、0x1410e0d10 でビートを間引いて BPM を 1/2 にする。拍番号の付け直しもある。これも範囲に入るまで繰り返す。
  - 最後にサンプル位置を `pos * 1000000 / sr` でミリ秒 (+ 端数) に直し、PQTZ と同じ 12 byte のエントリ (拍番号 u16, BPM×100 u16, 時刻 ms u32 ほか) にする。
- 高精度解析の DBN 自体は、設定に関係なく 70〜180 BPM 固定で探索する (上記の即値)。範囲設定は後段で倍/半分に折り返すだけと思われる。

### 観察: 決定性に関わる点

- 乱数の使用は見当たらない (TF の推論、Viterbi、補間はいずれも同じ入力なら同じ結果になる処理)。
- TF のスレッド数は `GetNativeSystemInfo().dwNumberOfProcessors` で、CPU のコア数に依存する。
- 旧解析器は FTZ を立てて計算する。
- 仮説:
  - TF の CPU 推論は、スレッド数や CPU の SIMD 命令セット (AVX2 / AVX-512 / FMA の有無) で浮動小数点の和の順序が変わり、出力確率がわずかに変わる可能性がある。
  - ただし後段は Viterbi の argmax と ms 丸めなので、わずかな差は吸収されることが多いと思われる。
  - 「別環境でも ANLZ が一致した例がある」という実データとは矛盾しないが、どの環境でもビット単位で一致するとは言い切れない。

## 2026-10-09 モデルの中身と madmom / beatcheck の照合 (メインセッションで実施)

### 方法

- `C:\Program Files\rekordbox\rekordbox 7.2.3\models\detect_beat\model7|model8` を scratchpad にコピーして読んだ。TensorFlow / protobuf は使わず、`keras_metadata.pb` の JSON と、`variables.index` (LevelDB 形式の SSTable と BundleEntryProto) を自作のパーサで読んだ。スクリプトは scratchpad の `scripts/tfckpt.py`、`cmp_models.py`、`cmp_full.py`、`cmp_filterbank.py`。
- 比較対象:
  - madmom (CPJKU/madmom 27f032e、models サブモジュール 7e3dc1b) の `madmom/models/downbeats/2016/downbeats_blstm_[1-8].pkl`。madmom のクラスはスタブで unpickle した。
  - beatcheck の `models/nc_reference/downbeats_blstm_weights.npz` (madmom オリジナルを変換したもの、追跡外) と `models/downbeats_blstm_weights.npz` (自前学習)。

### 観察 (事実)

- `keras_metadata.pb` の層構成: `TestLayerModel` (Keras 2.12.0) の中に、`STFTLayer` が 3 つ (frame_length 1024 / 2048 / 4096、frame_step 441) と `FilteredSpectrogram` が 3 つ。
- RNN の変数は 38 個。方向ごとに kernel [入力, 100]、recurrent [25, 100]、bias [100]、[25] のベクトルが 3 本 (peephole)。これが L0〜L2 の fwd / bwd の 6 組分あり、最後に出力層 [50, 3] / [3]。合計 99,003 パラメータ。madmom の 1 ネット分の総数 (99,003) と同じ。
- **重みの全数照合**: 100 列を i, f, c, o の順で 25 列ずつ切り、madmom の各ゲート (weights, recurrent_weights, bias, peephole_weights) と出力層に対応付けて比べた。
  - model7 と `downbeats_blstm_7.pkl`: 92 配列 / 99,003 パラメータで最大絶対差 0.0。
  - model8 と `downbeats_blstm_8.pkl`: 同じく最大絶対差 0.0。
  - beatcheck の nc_reference の net6 / net7 とも一致した (nc_reference が madmom の 1〜8 を net0〜7 として並べているため)。
  - beatcheck の自前学習の重みとは一致しない (最も近いものでも最大絶対差 0.65〜2.8)。
- **フィルタバンク**: madmom の `LogarithmicFilterbank` を RNNDownBeatProcessor と同じ設定 (fft_frequencies(frame/2, 44100)、num_bands 3/6/12、fmin 30、fmax 17000、norm_filters=True) で作った float32 行列 (512×21、1024×45、2048×91) が、両モデルの `saved_model.pb` にバイト単位でそのまま入っていた (model7: オフセット 1260658 / 325418 / 511160、model8: 74528 / 185070 / 374030)。バンド数の合計 157 × 2 (スペクトルと正の差分) = 314 で、LSTM の入力次元と一致する。
- DBN の設定の比較 (rekordbox 側は Ghidra の静的解析の結果で、動かしての確認はしていない):

  | | beats_per_bar | min/max BPM | num_tempi | transition_λ | observation_λ | threshold |
  |---|---|---|---|---|---|---|
  | madmom DBNDownBeatTrackingProcessor 既定 | 引数で指定 | 55 / 215 | 60 | 100 | 16 | 0.05 |
  | rekordbox 7.2.3 (Ghidra) | [4] | 70 / 180 | 60 | 100 | 16 | 0.05 |
  | beatcheck (madmom-beats-port 8ba4350 の CoreConfig 既定) | [3, 4] | 55 / 215 | 60 | 100 | 16 | 0.05 |

### 仮説 (推論)

- rekordbox の高精度ビート解析は、madmom の RNNDownBeatProcessor の前処理と、学習済みの BLSTM 8 本のうち 7 番と 8 番を、そのまま TensorFlow (Keras) に移したものと考えられる。重みとフィルタバンクがビット単位で一致するので、独自に再学習した可能性は低い。
- madmom はアンサンブル (8 本の出力の平均) だが、rekordbox は 1 本ずつ使う。Ghidra の結果と合わせると、通常解析とダイナミック解析で 7 番と 8 番を使い分けている可能性がある (どちらがどちらかは未確認)。
- 後段の DBN は madmom の既定値から BPM 範囲 (70〜180) と拍子 (4 のみ) を変えただけで、残りのパラメータは既定値のままと思われる。
- madmom のモデル / データファイルのライセンスは CC BY-NC-SA 4.0 (beatcheck の docs/retrain-madmom.md の記述による)。Pioneer / AlphaTheta が別途許諾を得ているかどうかは、このデータからは分からない。

## 2026-10-09 ライブラリの曲がどの解析器で解析されたか (未解決)

きっかけ: beatcheck の自前重みは rekordbox ライブラリのグリッドを教師にしている (beatcheck `docs/retrain-madmom.md`。gold は「キューあり or プレイリスト所属」、auto はそれ以外の自動解析のまま)。rekordbox の高精度解析が madmom の NC 重み (7 / 8) そのものなので、自動解析のままのグリッドが高精度解析で作られたものなら、教師データは NC 重みの出力を含むことになる。

### 観察 (事実)

- djmdContent.Analysed: 105 が 7484 曲、121 が 5 曲、233 が 1 曲。登録年別では 2024 年の 6485 曲がすべて 105。AnalysisUpdated は 1 が 6958 曲で、2〜9 が残り。
- `%APPDATA%\Pioneer\rekordbox6\rekordbox3.settings` (2026-10-09 時点の設定): `trackAnalysisMode=0`、`trackHiPrecisionBeatAnalysisMode=0`、`trackAnalysisProcessMode=0`、`trackAnalysisBpmRange=7`、`trackAnalysisSettingsNetworkAnalysis=1`。
- Ghidra の結果 (上): 解析モード (`[設定+0x2ca8]` / `[param+0x18]+0x60`) が 2〜4 なら高精度 (TF + DBN)、それ以外は旧来の DSP 解析器。

### 分かっていないこと

- 設定値 (`trackAnalysisMode` / `trackHiPrecisionBeatAnalysisMode`) と内部の解析モード 2〜4 の対応。
- `Analysed` の値 (105 / 121 / 233) の意味。105=0x69、121=0x79、233=0xE9 で、0x10 と 0x80 の bit が違う。
- 2024 年の一括取り込み時点の設定。現在の設定から過去の設定は分からない。
- 確かめる方法の候補:
  - Ghidra で、設定キーから内部モードへの変換と、Analysed を書き込む箇所を読む。
  - 自動解析のままの曲で、NC の net6 / net7 単体 + DBN (bpb [4]、70〜180) の出力と PQTZ がミリ秒単位で一致するかを実測する。
