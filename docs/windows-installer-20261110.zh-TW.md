# Windows 繁體中文安裝版

2026-11-10 的本機維護建置已完成 Windows x64 release 與 NSIS 安裝包，版本為 1.2.0。使用者回報完成安裝，並已核對安裝內容符合該次建置。

## 使用與建置

安裝介面及 App 固定文字為繁體中文，採目前使用者安裝模式，附上原始 LICENSE。安裝後可從 HGhub 捷徑開啟，不需開發工具或 Vite。

需要 Microsoft Edge WebView2；若目標電腦缺少 Runtime，安裝程式會嘗試下載官方安裝程式。此本機版本尚未加入 Windows 發行者數位簽章。

在工具齊備的 Windows 專案根目錄執行：

```powershell
.\hghub.cmd tauri build --bundles nsis
```

也可使用一般環境的 `pnpm tauri build --bundles nsis`。Tauri 會載入 Windows 專用設定；輸出在 `src-tauri/target/release/bundle/nsis/`。

## 已完成檢查

- TypeScript 型別及前端建置通過。
- 48 項 Rust 離線測試通過，6 項連線測試保留手動執行。
- 首頁／分類載入、錯誤與重試，播放來源及繁體中文介面回歸通過。
- 主程式為 x86-64 Windows GUI 執行檔，NSIS 為繁體中文，LICENSE 納入安裝內容。
- 已核對本機安裝版本及授權檔；主程式與原始 release 僅有 Tauri 預期的 NSIS 封裝識別標記差異。

先前 Windows 開發版已有使用者確認當次遊客播放可用；此次安裝包沒有另外操作真實影音播放驗收。

## 發佈

原始碼資料夾未包含安裝 EXE。準備發佈時，將該次安裝檔、SHA-256 與授權說明附於你自己的 GitHub Releases，並記錄版本及實際驗證範圍。

保留原授權與 Required Notice；操作及背景說明見 [GitHub 上傳說明](github-upload.zh-TW.md) 與 [Tauri Windows 文件](https://v2.tauri.app/distribute/windows-installer/)。
