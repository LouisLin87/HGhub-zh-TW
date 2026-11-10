# 本機開發

本專案使用 React／TypeScript／Vite 與 Tauri 2／Rust。Windows 可使用首次雙擊初始化流程；其他平台先安裝 Node.js 22、pnpm 10、Rust 1.90 以上及 [Tauri 目標平台工具](https://v2.tauri.app/start/prerequisites/)。

## 通用入口

在專案根目錄執行：

```bash
pnpm install --frozen-lockfile
pnpm app:dev
pnpm build
```

`app:dev` 啟動前端伺服器與原生 App；結束開發可關閉視窗並停止終端機。只有瀏覽器前端時，Tauri IPC 與原生媒體協定不可用，不能作為完整 App 驗證。

## Windows 專案內工具

直接雙擊 `hghub.cmd`，首次會自動建立前端及原生工具環境，再執行 `app:dev`。需要網路下載；第一次 Rust 編譯時間較長。發生錯誤時會保留畫面，不會直接關閉。

也可分別執行設定命令：

```powershell
.\hghub.cmd setup
.\hghub.cmd setup-native
.\hghub.cmd versions
.\hghub.cmd app:dev
```

`setup` 依腳本下載並核對 Node.js 官方檔案，再依鎖定檔安裝套件。`setup-native` 需要安裝 Microsoft C++ Build Tools、Windows SDK 與 Rust，可能出現系統安裝權限提示。WebView2 亦須可用。

`.tools/`、`node_modules/` 及快取由本機產生，未包含在 GitHub 上傳版；啟動入口設定的環境變數僅作用於其子程序。

## Windows 安裝包

```powershell
.\hghub.cmd tauri build --bundles nsis
```

輸出位於 `src-tauri/target/release/bundle/nsis/`。詳見 [Windows 安裝說明](windows-installer-20261110.zh-TW.md)。

## Android 與 iOS

- Android：另需 JDK、SDK、NDK 與 Rust target，依 [Android 設定](android-local-development.zh-TW.md) 建立本機設定。
- iOS：需要 macOS、完整 Xcode、對應 Rust target 及適用的簽章。範例 Team ID 必須依實際環境設定；此維護版尚未驗證 iOS 建置。

## 本機回歸

```powershell
$env:BROWSER_EXECUTABLE = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
.\hghub.cmd test:api-loading
.\hghub.cmd test:playback-source
.\hghub.cmd exec node scripts/test-locale-zh-tw.mjs
.\hghub.cmd exec cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

瀏覽器驗證使用本機模擬 IPC 與合成資料，外部要求會被封鎖。Rust 需連線的測試以 `#[ignore]` 保留，必須另行明確執行。

代理人設定位於 AGENTS.md 與 docs/agents/，任務沿用本機 `.scratch/`；已有適用設定時直接沿用。

首次啟動流程的回歸檢查可執行 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/test-startup-bootstrap.ps1`；測試使用假工具，不會下載 SDK 或啟動真實內容。
