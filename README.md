# Godot OSI Plugin

Godot 4 で [ASAM OSI](https://www.asam.net/standards/detail/osi/) (Open Simulation
Interface) を gRPC で受信し、Godot で扱いやすい形に変換するためのラッパー
GDExtension。要件の詳細は [REQUIREMENTS.md](REQUIREMENTS.md) を参照。

- 受信: `GroundTruthService.StreamGroundTruth` / `HostVehicleDataService.StreamHostVehicleData`（サーバーストリーミング gRPC）
- OSI バージョン: v3.7.0
- 実装: Rust (godot-rust / gdext + tonic + prost)、単一ネイティブ GDExtension
- 対象: Godot 4.6 / Windows x86_64

## ディレクトリ構成

```
Godot-OSI-plugin/
├─ Cargo.toml                # Rust ワークスペース
├─ rust-toolchain.toml       # ツールチェーン固定 (1.94.1)
├─ proto/
│  ├─ osi3/                  # ASAM OSI v3.7.0 の .proto 一式 (osi_version.proto は生成済み)
│  └─ service/               # gRPC サービス定義 (StreamGroundTruth / StreamHostVehicleData)
├─ crates/
│  ├─ osi-types/             # proto -> Rust 型 + tonic クライアント/サーバー stub (build.rs で生成)
│  └─ godot-osi/             # GDExtension 本体 (cdylib)。受信プラグイン + 変換プラグイン
├─ godot/                    # 開発/テスト用 Godot プロジェクト
│  └─ addons/godot_osi/
│     └─ godot_osi.gdextension
└─ temp/                     # 元の service proto と Godot 4.6.3 本体
```

## 前提

- Rust 1.94.1（`rust-toolchain.toml` で固定。インストール済みの rustup を利用）
- MSVC ツールチェーン（`x86_64-pc-windows-msvc`）
- protoc は不要。ビルド時に `protoc-bin-vendored` が同梱バイナリを使用
- Godot 4.6.x（本リポジトリでは `temp/Godot_v4.6.3-stable_win64.exe/` を使用）

## ビルド

```powershell
cargo build            # debug: target/debug/godot_osi.dll
cargo build --release  # release: target/release/godot_osi.dll
```

`godot/addons/godot_osi/godot_osi.gdextension` が `res://../target/{debug,release}/godot_osi.dll`
を参照する。

## Godot で開く

```powershell
& "temp\Godot_v4.6.3-stable_win64.exe\Godot_v4.6.3-stable_win64.exe" --path godot
```

エディタ起動時のログに次が出れば GDExtension のロード成功:

```
Initialize godot-rust (API v4.6.stable.official, runtime v4.6.3.stable.official, ...)
```

## 受信と変換をつなぐ（統合）

受信ノード `OsiReceiver` と変換ノード `OsiConverter` は内部の `OsiFrameBus`（最新フレーム
優先・`Arc` 共有）で繋がる。GDScript からは `OsiConverter.connect_source(receiver)` を呼ぶだけで
両者が同一バスを共有する（受信が producer、変換が consumer）。接続は `connect_to_server` の前後
どちらで呼んでもよく、再接続後も同じバスを使い続ける。さらに `OsiMovingObjectSpawner.bind_converter(converter)`
で変換結果の `MovingObject` を Node3D として自動生成できる。最小サンプルは
[godot/examples/osi_pipeline.gd](godot/examples/osi_pipeline.gd)（`OsiMockServer` を使う自己完結版にもできる）。

```gdscript
var receiver := OsiReceiver.new();  add_child(receiver)
var converter := OsiConverter.new(); add_child(converter)
converter.connect_source(receiver)   # 同一 OsiFrameBus を共有
receiver.connect_to_server()
converter.ground_truth_converted.connect(func(snap): print(snap.moving_object.size()))
```

結合テスト（モックサーバー → 受信 → 変換 → シグナル）は headless で:

```powershell
pwsh godot/test/run_integration.ps1
```

## ビジュアルデモ（画面で見る）

[godot/examples/osi_visual_demo.tscn](godot/examples/osi_visual_demo.tscn) を Godot で開いて実行すると、
受信した `MovingObject` を**色付きのボックス**で可視化できる（OSI の `dimension` でサイズ、
type で色分け＝車両:青/歩行者:緑/動物:橙、yaw で向き）。`OsiMovingObjectVisualizer` が
変換結果から `MeshInstance3D` を id 追跡で生成・更新・破棄する。

```powershell
cargo build
# シーンの use_mock=true（既定）なら外部サーバー不要。バンドルのモックが車両2台＋歩行者1人を
# 円運動でストリームする。実サーバーを見るなら use_mock=false にして host/port を指定。
& "temp\Godot_v4.6.3-stable_win64.exe\Godot_v4.6.3-stable_win64.exe" --path godot godot/examples/osi_visual_demo.tscn
```

ボックス生成の headless スモークテスト:

```powershell
& "temp\...\Godot_..._console.exe" --headless --path godot --script res://test/visual_smoke.gd
```

## 配布パッケージ

リリース（`v*` タグの push、または Actions の手動実行）で
[.github/workflows/release.yml](.github/workflows/release.yml) が **2 種類の zip** を生成する:

| zip | 中身 |
|---|---|
| `godot-osi-<ver>.zip` | ランタイム一式（サンプル無し） |
| `godot-osi-<ver>-examples.zip` | 上記に `addons/godot_osi/examples/` を追加 |

addon は **ランタイム専用**（`plugin.cfg` 無し）。展開して `res://addons/godot_osi/` に置くだけ。
同梱されるドキュメントは利用者向けで、Godot エディタ補完に頼れない環境（Claude Code 等）でも
API を完全に把握できることを意図している:

- `README.md` … インストール + クイックスタート + 配線 + 座標系の注意（[packaging/README.md](packaging/README.md)）
- `API.md` … 公開ノードクラスのリファレンス（[packaging/API.md](packaging/API.md)）
- `SCHEMA.md` … 全 `Osi*` 型付き Resource とフィールドの一覧。**`build.rs` がビルド時に
  `packaging/generated/SCHEMA.md` へ自動生成**（同じレジストリから生成するので proto と常に同期）。
- `THIRD_PARTY_NOTICES.md` / `LICENSE-*` / `third_party/osi3/*.proto`

## ライセンス

このリポジトリには2系統のライセンスが混在します。MPL-2.0 は**ファイル単位の弱コピーレフト**
（GPL のような「リンクで全体に感染」ではない）なので、両者は矛盾なく共存します。

### 1. 手書きの独自コード → `MIT OR Apache-2.0`（デュアル）

`proto/service/`、`crates/godot-osi/` の手書きソース、ビルドスクリプト等は
**`MIT OR Apache-2.0`** のデュアルライセンスです。利用者は好きな方を選べます。
- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)

### 2. MPL-2.0 由来の部分（自動的に MPL-2.0 のまま）

次のものは MPL-2.0 で保護されており、上記デュアルの対象**外**です:

- **`proto/osi3/` の ASAM OSI `.proto` 定義**（Copyright BMW AG ほか / [proto/osi3/LICENSE](proto/osi3/LICENSE)）。
  改変して再配布する場合は当該ファイルを MPL-2.0 のまま公開する必要があります。
- **`crates/osi-types` がビルド時に `.proto` から生成する Rust 型**。MPL の `.proto` の派生物
  （MPL でいう Modification）に当たるため、生成物も実質 MPL-2.0 として扱うのが安全です。
- **gdext (godot-rust)** … MPL-2.0。リンクしても自分の手書きファイルには感染しませんが、
  バイナリには MPL コードが含まれます。

### 3. 配布バイナリ（GDExtension `.dll`）の義務

ビルド成果物は上記 MPL 部分（gdext + OSI 由来の生成型）を含む **"Larger Work"** です。
バイナリを再配布する際は、**MPL-2.0 が対象とするファイル（gdext のソースと OSI `.proto`）の
入手手段を提供する義務**が残ります。本リポジトリの配布形態（ソース同梱 + 依存はクレート経由）
であればこの要件は満たされます。利用者自身のアプリ／ゲームコードは proprietary のままで構いません。

依存クレートはいずれも商用利用可（tonic=MIT, prost=Apache-2.0, gdext=MPL-2.0, Godot 本体=MIT）。
詳細は [REQUIREMENTS.md](REQUIREMENTS.md) の §7 を参照。

## 現状と次の作業

- [x] OSI v3.7.0 proto 取得・配置
- [x] Rust ワークスペース + gdext 雛形
- [x] proto -> Rust 型生成（osi-types ビルド成功）
- [x] GDExtension cdylib ビルド + Godot 4.6.3 ロード検証
- [ ] 受信プラグイン: tonic gRPC クライアント + バックグラウンドスレッド + signal
- [ ] 変換プラグイン: proto -> Godot 型付き Resource のコードジェネレータ + 座標変換
- [ ] モック gRPC サーバー + OSI トレース再生
- [ ] サンプル: MovingObject の Node3D 同期ヘルパー
```
