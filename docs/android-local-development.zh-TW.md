# Android 本機開發與測試

本維護版已在 Windows 完成 ARM64 測試 APK 的編譯、封裝、簽章與對齊檢查，手機實機驗證仍待完成。使用獨立套件名稱 `com.hghub.huangguo.local`、名稱「HGhub 測試版」，可與原版並存。

## 建置工具

目前本機建置使用的版本為 JDK 17、SDK Platform 36、Build Tools 36.0.0、NDK 28.2.13676358，以及 Rust target `aarch64-linux-android`。環境需求請同時參考 [Tauri 官方文件](https://v2.tauri.app/start/prerequisites/#android)。

Windows 的 Android 原生工具對中文路徑有限制。JDK、SDK、NDK、Android 用的 Rustup／Cargo 資料及建置副本應放在 ASCII 英文路徑；腳本會同步原始碼至獨立的英文建置目錄。

## 設定本機路徑

1. 安裝上述工具與對應 Rust target。
2. 建立 `.tools/`，將 `examples/android-local-env.example.ps1` 複製為 `.tools/android-local-env.ps1`。
3. 將範例內工具及建置位置改成自己的實際英文路徑。若採獨立的 RUSTUP_HOME／CARGO_HOME，須先在該位置建立可用工具鏈並安裝 Android target。
4. 先完成前端相依套件安裝，再執行建置入口。

```powershell
.\hghub.cmd setup
.\hghub-android.cmd
```

範例設定不包含 SDK、Rust 工具、簽章或任何帳號資料，不能只複製設定檔就視為環境已就緒。

## 輸出與簽章

建置入口會顯示 APK 路徑及 SHA-256；範例設定輸出至專案內 `artifacts/android/`。該目錄由 `.gitignore` 排除，安裝檔可另放 Releases。

目前入口產生 ARM64 debug 測試包，最低 API 24、目標 API 36，內嵌前端資源。App 不需要電腦持續執行開發伺服器，內容服務仍需要網路。

Windows 缺少 JNI 符號連結權限時，腳本僅在確認該次 Rust 編譯成功及遇到指定錯誤後，改用一般檔案複製，再交由 Gradle 封裝。封裝副本移除除錯資訊，完整編譯檔留在 target/。此流程不變更 Windows 安全設定。

測試金鑰會保存於本機 Android 使用者目錄，另備份至 `.tools/android-signing/debug.keystore`。更新相同套件需沿用相同憑證；請保持這些檔案私有。正式簽署與發佈需另行規劃。

## 手機驗證

將 APK 傳到手機安裝，先確認主畫面及遊客播放，再測試時間、聲音、全螢幕、旋轉、返回與前景恢復。靜態封裝檢查不能代替實機驗收。

需要 ADB 診斷時可執行：

```powershell
.\hghub-android.cmd devices
```

單純透過手機檔案管理員安裝 APK 不需要 USB 偵錯。
