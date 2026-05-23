# アーキテクチャと並列作業ガイド

単一の GDExtension（`crates/godot-osi`、1 つの `godot_osi.dll`）に 2 つのプラグインを同居させる。
両者は **生 OSI フレームという 1 点の境界**だけで接し、独立に開発・テストできる。

```
 gRPC server / mock                                  Godot scene
        │                                                  ▲
        ▼                                                  │ signal (typed Resources)
 ┌──────────────┐   OsiFrameBus (newest-wins)   ┌──────────────────┐
 │ OsiReceiver  │ ───────────────────────────▶ │ OsiConverter      │
 │ (receiver.rs)│   osi3::GroundTruth /         │ (converter/)      │
 │              │   osi3::HostVehicleData       │ + coord 変換      │
 └──────────────┘   = prost types (osi-types)   └──────────────────┘
   feature/receiver                                feature/converter
```

## 境界（唯一の共有契約）

`crates/godot-osi/src/frame_bus.rs` の `OsiFrameBus`。
- 受信側（producer）が `bus.ground_truth.store(frame)` で最新フレームを書く。
- 変換側（consumer）が `bus.ground_truth.take()` で取り出す。
- 「最新フレームのみ採用」: `store` は未消費の旧フレームを破棄する。
- 入力型は `osi_types::osi3::GroundTruth` / `HostVehicleData`（prost 生成型）。

この型は `osi-types` に既に存在するため、**両プラグインは互いに依存しない**。

## モジュール所有権（衝突回避）

| パス | 所有セッション | 備考 |
|---|---|---|
| `crates/godot-osi/src/frame_bus.rs` | 共有（main で確定済み・原則変更しない） | 変更時は両者で合意 |
| `crates/godot-osi/src/receiver.rs` | `feature/receiver` | gRPC クライアント + `OsiReceiver` |
| `crates/godot-osi/src/converter/**` | `feature/converter` | コードジェネレータ + `OsiConverter` |
| `crates/godot-osi/src/lib.rs` | 共有（`mod` 宣言は確定済み） | 基本触らない |
| `crates/godot-osi/Cargo.toml` | 共有 | 依存追加で軽微な衝突あり得る（`cargo add` は追記なので解消容易） |
| `crates/godot-osi/build.rs` | `feature/converter` が新規作成 | 受信側は作らない＝衝突しない |
| `proto/`, `crates/osi-types/` | 確定済み | 原則変更しない |

`receiver.rs` / `converter/mod.rs` の `mod` 宣言と、受信側の実行時依存（`tokio`, `tonic`）は
main に先入れ済み。各セッションは原則として自分の所有パスだけを編集すれば衝突しない。

## 独立テストの仕方

- **変換側**: 受信も Godot も不要。`osi3::GroundTruth` を手で組み立てて
  `convert_*` 関数（純ロジックは Godot クラスから分離して書く）を `cargo test` で検証。
- **受信側**: 変換は不要。バンドルするモック gRPC サーバー（既定 `127.0.0.1:50051`）に対して
  接続し、`OsiFrameBus` にフレームが入ることを確認。GDExtension のロードは
  `temp/Godot_v4.6.3-stable_win64.exe/` で検証。

## ブランチ / worktree 運用

- `feature/receiver` と `feature/converter` をそれぞれ git worktree で開く（別ディレクトリ）。
- 各セッションはこまめに `main` をマージして境界の変化を取り込む。
- 完成したら PR で `main` へマージ。`lib.rs` / `Cargo.toml` の競合は軽微。
- 統合作業（同一 `OsiFrameBus` インスタンスを両ノードに渡す配線）は両者マージ後に行う。

## 統合時の配線（後工程の覚書）

`OsiConverter` に `set_source(receiver: Gd<OsiReceiver>)` のようなメソッドを設け、
`receiver` が持つ `OsiFrameBus` のクローン（`Arc` 共有）を受け取る。これで 1 つのバスを
producer/consumer が共有する。GDScript からは receiver と converter を並べて接続するだけ。
