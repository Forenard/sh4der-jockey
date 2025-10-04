# Spoutサンプル

このサンプルは、レンダリング出力をSpout経由で他のアプリケーションと共有する方法を示します。

## 必要要件

- Windows OS（SpoutはWindows専用です）
- OpenGL対応グラフィックスカード
- **SpoutLibrary.dll** - インストール手順は [SPOUT_SETUP.md](../../SPOUT_SETUP.md) を参照
- Spout受信アプリケーション（テスト用、オプション）:
  - [SpoutReceiver](https://leadedge.github.io/)
  - Resolume
  - TouchDesigner
  - その他のSpout対応アプリケーション

## ⚠️ 重要: SpoutLibrary.dllが必要です

Spout受信側とテクスチャを実際に共有するには、**SpoutLibrary.dllをインストールする必要があります**。

詳しいインストール手順は [../../SPOUT_SETUP.md](../../SPOUT_SETUP.md) を参照してください。

このDLLがない場合、アプリケーションは動作しますが、受信側で"Sh4derJockey"送信元を見ることができません。

## セットアップ

**重要**: まず[SPOUT_SETUP.md](../../SPOUT_SETUP.md)に従って`SpoutLibrary.dll`をプロジェクトルートの`libs/`ディレクトリに配置してください。

1. DLLを配置:
   ```bash
   # プロジェクトルートに libs/SpoutLibrary.dll を配置
   # ビルド時に自動的に target/debug/ にコピーされます
   ```

2. プロジェクトルートから実行:
   ```bash
   # プロジェクトルートで
   cargo run
   ```

3. または、このディレクトリから実行:
   ```bash
   cd example/spout
   cargo run
   ```

4. アプリケーションは以下を行います:
   - カラフルなアニメーションパターンを表示
   - 出力を"Sh4derJockey"としてSpoutに送信

5. Spout受信アプリケーションを開いて、共有されたテクスチャを確認

## 設定

`pipeline.yaml`ファイルにSpout設定が含まれています:

```yaml
spout:
  enabled: true        # Spout送信の有効/無効
  name: "Sh4derJockey" # Spout送信名（受信アプリに表示される）
```

### 設定オプション

- **`enabled`** (boolean): `true`でSpout送信を有効化、`false`で無効化
- **`name`** (string): 受信側に表示されるSpout送信元の名前

## デバッグ

Spoutのデバッグ出力を表示するには、ログを有効にして実行:

```bash
# Windowsコマンドプロンプト
set RUST_LOG=info
cargo run

# より詳細なログの場合
set RUST_LOG=debug
cargo run
```

以下のようなメッセージが表示されます:
- `Spout sender 'Sh4derJockey' initialized` - 送信元が作成されたとき
- `Sent texture {id} ({width}x{height}) to Spout sender '{name}'` - 各フレーム（traceレベル）

## 仕組み

1. **初期化**: パイプライン読み込み時、`spout.enabled: true`の場合にSpoutSenderが作成されます
2. **フレームレンダリング**: 各フレームがウィンドウにレンダリングされます
3. **テクスチャ共有**: レンダリングされたフレームバッファがテクスチャにコピーされ、Spout経由で共有されます
4. **受信側アクセス**: 他のアプリケーションが"Sh4derJockey"送信元をリアルタイムで受信できます

## 実装の詳細

- OpenGLテクスチャコピー（`glCopyTexImage2D`）でフレームバッファをキャプチャ
- OpenGL相互運用でテクスチャを共有（簡易実装）
- 最小限のパフォーマンスオーバーヘッド（テクスチャコピーのみ）
- 解像度変更を自動的に処理

## トラブルシューティング

**受信アプリで送信元が見えない:**
- `pipeline.yaml`で`enabled: true`になっているか確認
- アプリケーションが実行中か確認
- 送信名が一致しているか確認
- SpoutLibrary.dllがインストールされているか確認（[SPOUT_SETUP.md](../../SPOUT_SETUP.md)参照）

**パフォーマンスの問題:**
- テクスチャコピーには多少のオーバーヘッドがあります
- 最高のパフォーマンスを得るには、ウィンドウ解像度を受信側の期待する解像度に合わせてください
- 不要な場合は`enabled: false`でSpoutを無効化することを検討してください

**SpoutLibrary.dllに関するエラー:**
- DLLのインストール手順を [SPOUT_SETUP.md](../../SPOUT_SETUP.md) で確認
- ログで`Using SpoutLibrary.dll for Spout sending`が表示されるか確認
- `Falling back to basic OpenGL implementation`が表示される場合、DLLが見つかっていません
