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

## ライセンス

このリポジトリの**独自コード**（`crates/`, `proto/service/`, ビルドスクリプト等）は
**`MIT OR Apache-2.0`** のデュアルライセンスです。利用者は好きな方を選べます。
- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)

ただし `proto/osi3/` に同梱している **ASAM OSI の .proto 定義は MPL-2.0**（Copyright BMW AG ほか）
であり、その条件下で再配布しています（[proto/osi3/LICENSE](proto/osi3/LICENSE)）。
これらを改変して再配布する場合は、当該ファイルを MPL-2.0 のまま公開する必要があります。

ビルド成果物（GDExtension バイナリ）は上記 OSI 定義から生成された型を含むため、
MPL-2.0 で保護された OSI ソース（同梱の .proto）の入手手段を伴って配布されます。

依存クレートはいずれも商用利用可（tonic=MIT, prost=Apache-2.0, gdext=MPL-2.0）。
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
