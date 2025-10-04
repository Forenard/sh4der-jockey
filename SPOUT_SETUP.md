# Spoutセットアップ手順

完全なSpout機能とテクスチャ共有を有効にするには、Spout Library DLLをインストールする必要があります。

## 必要要件

- Windows OS
- OpenGL対応グラフィックスカード
- Visual C++ 再頒布可能パッケージ（通常は既にインストール済み）

## インストール手順

### 1. Spout Libraryのダウンロード

最新のSpout SDKバイナリを以下からダウンロード:
https://github.com/leadedge/Spout2/releases

`Spout-SDK-binaries_<version>.zip`という名前のファイルを探してください（例: `Spout-SDK-binaries_2-007-015.zip`）

### 2. SpoutLibrary.dllの抽出

ダウンロードしたZIPファイルから以下を抽出:
- `Libs/MD/bin/SpoutLibrary.dll`（MD版 - 動的リンク、推奨）

**注意**: `Libs/MT/bin/`にもSpoutLibrary.dllがありますが、**MD版を使用してください**。
- **MD (Multi-Threaded DLL)**: 動的リンク版（Rust製アプリに適合）
- **MT (Multi-Threaded)**: 静的リンク版（使用しない）

### 3. DLLの配置

`SpoutLibrary.dll`を以下の場所にコピー:

**libs/ディレクトリに配置（推奨）**
```
sh4der-jockey/
├── libs/
│   └── SpoutLibrary.dll  <-- ここに配置
├── target/  # ビルド時に自動コピーされます
│   ├── debug/
│   │   └── SpoutLibrary.dll  (自動生成)
│   └── release/
│       └── SpoutLibrary.dll  (自動生成)
```

**注意**:
- `libs/SpoutLibrary.dll`に配置すれば、ビルド時に自動的に`target/debug/`や`target/release/`にコピーされます
- `cargo clean`しても`libs/`ディレクトリのDLLは消えません
- 手動で`target/debug/`や`target/release/`に直接配置することも可能ですが、clean時に削除されます

### 4. インストールの確認

Spoutを有効にしてsh4der-jockeyを実行すると、以下のように表示されます:
```
INFO  sh4der_jockey::jockey::spout > Using SpoutLibrary.dll for Spout sending
INFO  sh4der_jockey::jockey::spout > Spout sender 'Sh4derJockey' initialized (1280x720)
```

DLLが見つからない場合は以下のように表示されます:
```
WARN  sh4der_jockey::jockey::spout > Failed to initialize SpoutLibrary: ...
WARN  sh4der_jockey::jockey::spout > Falling back to basic OpenGL implementation
```

## トラブルシューティング

### "Failed to load library" (ライブラリの読み込みに失敗)

- **x64**版をダウンロードしたか確認してください（x86ではありません）
- Visual C++ 再頒布可能パッケージをインストール: https://aka.ms/vs/17/release/vc_redist.x64.exe

### "Library found but fails to initialize" (ライブラリは見つかったが初期化に失敗)

- WindowsイベントビューアでDLL読み込みエラーを確認
- グラフィックスドライバが最新であることを確認
- 管理者として実行してみる

### "Sender not visible in receivers" (受信側で送信元が見えない)

- SpoutLibrary.dllが読み込まれているか確認（ログメッセージを確認）
- 送信側と受信側のアプリケーションが同じアーキテクチャ（64bit）で動作しているか確認
- 両方のアプリケーションを再起動

## フォールバックモード

SpoutLibrary.dllが利用できない場合、sh4der-jockeyはフォールバックモードで動作します:
- アプリケーションは正常に実行されます
- テクスチャは内部で処理されます
- **Spout受信側は送信元を見ることができません**（これは制限事項です）

完全なSpout機能を使用するには、**必ずSpoutLibrary.dllをインストールしてください**。

## ソースからのビルド

Spoutサポートをソースからビルドする場合、以下の方法もあります:

1. Spout2リポジトリをクローン:
   ```bash
   git clone https://github.com/leadedge/Spout2.git
   ```

2. Visual Studioを使用してSpoutLibraryをビルド（VS 2019以降が必要）

3. ビルドした`SpoutLibrary.dll`を上記の手順に従って配置
