# HGhub 繁體中文維護版

以 [Minireel/HGhub](https://github.com/Minireel/HGhub) 為基礎的個人非商業維護版本，包含繁體中文介面、遊客播放相容性修正及本機打包支援。

## 專案來源與致謝

感謝 HGhub 原作者及所有貢獻者提供原始專案。此版本的原始來源為 [Minireel/HGhub](https://github.com/Minireel/HGhub)；原始說明保存在 [README.upstream.md](README.upstream.md)。

這是個人維護分支，維護內容及版本發佈由此分支的維護者負責。保留原專案的著作權、授權及既有聲明。

## 本維護版的修改

- 導覽、播放器、設定、搜尋、登入及個人頁的固定文字繁體中文化。
- 遊客播放改用現行加密播放通道，保留來源、授權標頭與快取隔離。
- 返回相同影片或集數時重新取得短效播放來源。
- 熱門題材顯示繁體標籤，保留服務使用的既有搜尋值。
- 網路與播放診斷兼容既有及繁體中文錯誤訊息。
- Windows x64 繁體中文 NSIS 安裝包，包含原始 LICENSE。
- Android ARM64 本機測試版建置入口。

外部服務回傳的標題與內容保留來源資料。技術變更見 [CHANGELOG.md](CHANGELOG.md) 及 [遊客播放修正紀錄](docs/playback-guest-fix-20261110.zh-TW.md)。

## 已驗證範圍

| 平台或檢查 | 目前狀態 |
| --- | --- |
| Windows 開發版 | 已編譯、開啟，並由使用者確認當次遊客播放可用 |
| Windows x64 安裝版 | 已完成 release／NSIS 打包及本機安裝核對 |
| Android ARM64 | 已編譯與封裝，簽章、ABI、原生檔案及對齊檢查通過；實機待驗證 |
| iOS | 保留原專案支援與打包腳本；此維護版尚未完成編譯及實機驗證 |
| macOS／Linux | 保留原始碼與工作流程；此維護版尚未在這些平台驗證 |
| 本機回歸 | 型別檢查、48 項 Rust 離線測試、播放來源、載入／重試與繁體介面檢查通過 |

以上為 2026-11-10 的本機驗證紀錄。外部服務是否可用仍由服務方決定。

## Windows 首次開啟

直接雙擊根目錄的 `hghub.cmd`。第一次會建立 Node.js、pnpm、相依套件及 Rust 原生環境，再啟動桌面 App；需要網路，下載與首次編譯可能需數分鐘，請保持終端機開啟。環境齊備後再次雙擊會直接啟動。

設定或啟動失敗時，視窗會保留錯誤文字供排查。Windows 仍需可用的 WebView2；缺少 C++ Build Tools 時可能出現系統安裝權限提示。

## 開發與打包

基本需求為 Node.js 22、pnpm 10、Rust 1.90 以上，以及目標平台的 [Tauri 開發工具](https://v2.tauri.app/start/prerequisites/)。

```bash
pnpm install --frozen-lockfile
pnpm app:dev
pnpm build
pnpm tauri build --bundles nsis
```

Windows 另提供專案內工具入口：

```powershell
.\hghub.cmd setup
.\hghub.cmd setup-native
.\hghub.cmd app:dev
```

手機 App 使用原生能力，完整功能需要依目標平台建置。開發方式見 [本機開發說明](docs/local-development.zh-TW.md)、[Windows 安裝包](docs/windows-installer-20261110.zh-TW.md) 與 [Android 設定](docs/android-local-development.zh-TW.md)。

## GitHub 上傳與版本發佈

上傳步驟見 [GitHub 上傳說明](docs/github-upload.zh-TW.md)。原始碼與安裝附件分開管理：原始碼提交至儲存庫，安裝 EXE／APK 另附於 Releases。

此副本的 GitHub Actions 僅供手動觸發，建置結果保存為工作流程附件，不會自動建立或改寫 Release。原工作流程另存於 [上游工作流程原文](docs/upstream/workflows/)，供對照。這些工作流程尚未在你的 GitHub 儲存庫執行驗證。

## 授權與使用範圍

原專案採用 [PolyForm Noncommercial 1.0.0](LICENSE)。使用、修改及散布須遵守原授權；本維護版的目前範圍為個人非商業用途。

Required Notice: Copyright (c) 2026 HGhub contributors (https://github.com/Minireel/HGhub)

完整聲明見 [LICENSE](LICENSE)、[NOTICE.md](NOTICE.md) 及原始 README。程式碼授權不代表取得外部內容或服務的授權。請尊重原作者、內容權利人及相關服務的使用條件。
