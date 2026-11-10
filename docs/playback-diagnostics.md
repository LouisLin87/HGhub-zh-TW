# 播放診斷日誌

設定 → 播放診斷 → **播放診斷日誌**。預設關閉，開關在本機持久化，釋出版也可使用。

## 復現與取日誌

1. 開啟日誌。如需排查啟動時的後臺播放，開啟後退出並重新啟動應用。
2. 依次復現：播放首頁影片、切到其他頁面播放、點選進度條、拖動進度條。點選/鬆手後等待畫面恢復；若一直卡住，至少等待 10 秒。
3. 回到設定關閉日誌，點選 **複製播放診斷日誌**。關閉後已有日誌保留。

macOS 檔案位於 `~/Library/Application Support/com.hghub.huangguo/`：

- `playback.log`：當前診斷日誌，JSON Lines 格式。
- `playback.previous.log`：輪轉前的日誌，若存在，手動取檔案時請一併提供。
- `playback-diagnostics.json`：開關配置，與 `security.json` 獨立。

複製按鈕包含當前與上一份日誌。每份上限 2 MiB，總計約 4 MiB，自動輪轉。日誌只存本機；媒體地址用跨前後端一致的資源編號替代，不記錄播放直鏈、查詢引數、金鑰內容、賬號憑據或響應正文。原有 `debug.log` 仍用於應用異常與舊自檢，不受此開關控制。

## 如何讀記錄

每行包含 UTC `ts`、程序 `pid`、遞增 `seq`、當時生效的安全模式 `strict`、`event` 與 `data`。前端記錄還包含採集時刻 `data.at`、單調時鐘 `mono_ms`、頁面會話 `session`、播放器 `player`、頁面來源、影片/集數及啟用狀態。前端約每 250ms 批次寫入，分析事件先後順序使用 `data.at/mono_ms`，不只看落盤時間。

`debug.log` 新寫入的所有記錄也使用 RFC 3339 UTC（`Z` 結尾），格式為 `[2026-11-08T14:30:00.123Z] [pid=12345] [api] ...`。時間由後端統一在寫入時生成，覆蓋 API 錯誤、執行異常、未處理的 Promise 拒絕、網路自檢和播放自檢；多行正文/堆疊的每一行帶同一個時間和程序編號，同一條記錄的各行連續寫入。`playback.log` 的後端 `ts` 與前端 `data.at` 同樣都是 RFC 3339 UTC，精度可能不同；`mono_ms`、`elapsed_ms` 等為相對耗時，繼續保留毫秒數值。已有無時間戳的歷史記錄保留原樣，無法補出真實發生時間。

點選、拖動和鍵盤定位會記錄：

| 事件 | 含義 |
| --- | --- |
| `web.seek.input.down/change/up/cancel` | 輸入操作，包括 click/drag/keyboard、目標時間；點選同一位置即使沒有 change，也保留按下/鬆開 |
| `web.seek.request` | 真正發起定位，包含播放器內遞增的 `seek` 編號、目標時間和來源 |
| `web.seek.assigned` / `web.seek.rejected` | 寫入 currentTime 後的狀態，或寫入失敗 |
| `web.media.seeking` / `web.media.seeked` | 媒體元素確認正在定位/定位結束及耗時 |
| `web.seek.resumed` | 播放中定位後的首幀回撥；不支援首幀 API 時標記為 playing 事件回退 |
| `web.seek.ready` | 暫停定位完成、資料已就緒；保持暫停，不把它記作恢復播放 |
| `web.seek.superseded` | 之前的定位被下一次拖動/點選替代 |
| `web.seek.noop` | 目標與當前時間相同，無需等待 seeking 事件 |
| `web.seek.timeout` | 8 秒後仍未完成，包含目標、實際時間、緩衝/可定位範圍、readyState、paused、seeking 和 HLS 載入/緩衝開關 |

使用 `(pid, session, player, seek)` 關聯一次操作。拖動期間的 `seek.input.change` 記錄預覽位置，鬆手後才發起一次 `seek.request` 和 currentTime 寫入；點選在鬆開時定位，鍵盤定位立即執行。取消拖動或切到其他頁面會丟棄預覽，不發起跳轉。播放器每 5 秒記錄狀態快照，非啟用頁面仍播放時可看到 `active: false` 與 `paused: false`。

播放器在頁面失活時暫停並停止 HLS 分片載入；清單稍後返回時也必須重新檢查活動狀態。重新進入頁面時，之前正在播放的內容可繼續播放；手動暫停的內容保持暫停。共用播放頁與吃瓜詳情的獨立播放器均遵循此規則。

hls.js 固定為 **1.6.19**，包含 [macOS/iOS 27 ManagedMediaSource 跳轉恢復緩衝的上游修復](https://github.com/video-dev/hls.js/pull/7984)。兩個播放器均顯式配置 `preferManagedMediaSource: false`，在標準 MediaSource 可用時優先使用它；只有 ManagedMediaSource 的環境仍可回退，並由升級後的庫在 seeking 時恢復緩衝。診斷日誌中的 `hls_version`、`engine` 和 `prefer_managed` 可確認實際執行版本和路徑。

`web.hls.attached`/狀態快照記錄 MediaSource 型別與偏好；`web.mms.startstreaming/endstreaming`、分片載入/緩衝、非致命和致命 HLS 錯誤也會記錄。`protocol.request` 包含原請求的 Range 和 Origin 分類；`proxy.begin/end` 與 `network.begin/end` 通過 `resource` 和請求編號關聯代理耗時、HTTP 狀態、位元組數與每次重試。`network.target_failed`、`security.origin_denied`、`web.security.csp_violation` 用於識別安全拒絕，不需要關閉安全保護。

業務介面也受同一個日誌開關控制：`api.begin/end` 記錄請求編號 `request`、每次嘗試 `attempt`、操作型別 `operation`（首頁/分類/播放地址等）、邏輯方法、所選線路 `line_host`、重定向後目標 `target_host`、HTTP 狀態、響應頭/總耗時、響應型別與閘道器類別。`business_status` 是 HTTP 200 後返回的業務狀態；`retryable` 表示當前邏輯請求及錯誤是否允許重試。通過 `(pid, request, attempt)` 關聯一次嘗試，結合 `line.changed` 和每行的 `strict` 判斷是否與切換線路或安全模式有關。只記錄主機名和固定操作名，不記錄搜尋詞、賬號、內容 ID、請求體、憑據或響應正文。舊版只有 `[api]` 錯誤的 `debug.log` 無法事後補出請求時間和線路。

HTTP 404/其他不可重試的 4xx、許可權和安全拒絕直接結束；超時、408/425/429 和 5xx 的邏輯只讀請求仍最多嘗試三次。每次請求保留原有 20 秒超時，因此持續 5xx/超時時仍可能等待較久；日誌會逐次記錄。首頁失敗後顯示重試入口；分類首次開啟才請求預覽，各頻道和帖子獨立更新、獨立報錯及重試，避免一個慢請求拖住整頁。分片計時尚未就緒時記錄 `null`，不再出現負耗時。

可執行 `pnpm test:api-loading` 注入首頁 404、頻道 503、慢頻道和空結果。測試使用真實頁面及 API 錯誤處理，替換 Tauri IPC，阻止所有外部請求；檢查首頁停止轉圈並可重試、成功頻道與帖子提前顯示、失敗頻道單獨重試，以及未開啟分類時不預取。結果儲存到 `test-results/api-loading/<UTC時間>/`。Rust 網路層另行測試 404/許可權/安全拒絕只嘗試一次，503/超時可以重試恢復。

## 本地驗證

使用 [進度條迴歸的媒體樣本與伺服器](playback-seek.md)，開啟 `http://127.0.0.1:1420/tests/player-seek.html?diagnostics=1`。該模式使用真實 Feed、PlayerControls、VideoSurface 和 hls.js，日誌傳輸替換為記憶體；“診斷設定”按鈕提供真實設定介面。控制台的 `window.playerSeekFixture.logs` 可檢視記錄，`flushLogs()` 可立即重新整理批次。

驗證預設關閉時點選不記錄；通過設定開啟後，實際滑鼠點選和拖動應分別產生 `desktop_click`、`desktop_drag`，並串聯 request、seeking、seeked、首幀恢復。拖動時有多個預覽記錄但沒有實際定位，鬆手後恰好定位一次。暫停定位和鍵盤定位應產生 ready 且保持暫停；關閉後不再增加記錄；儲存失敗保持原狀態。前後端型別檢查、Rust 日誌脫敏/輪轉/儲存失敗測試也隨構建檢查。

追加 `&legacy=1` 可以檢查吃瓜詳情頁使用的獨立播放器。

本地 Chromium 驗證不替代 macOS WKWebView 實機復現。

## 無頭伺服器自動採集

需要 Node.js 22+、pnpm、FFmpeg（含 libx264/AAC）及 Chromium。首次準備：

```sh
pnpm install --frozen-lockfile
pnpm exec playwright-core install --with-deps chromium
```

Ubuntu 上可用 `sudo apt-get install ffmpeg` 安裝媒體樣本生成器，然後執行：

```sh
pnpm test:playback
```

命令自動生成 90 秒 HLS 樣本、啟動僅監聽本機隨機埠的 Vite/媒體服務、操作無頭瀏覽器，並在結束後關閉服務、瀏覽器及刪除臨時媒體。不需要顯示器、桌面會話、賬號或播放直鏈。測試中的外部網路請求會被攔截並導致失敗。

每次執行儲存到新的 `test-results/playback/<UTC時間>/` 目錄，包含：

- `report.json`：平臺、瀏覽器版本、每個檢查的結果、點選耗時、拖動請求數及失敗原因。
- 各場景的 `playback.jsonl`：邊操作邊落盤的播放器診斷日誌，採集不依賴手動複製或測試成功結束。
- `network.jsonl`、`console.jsonl`：瀏覽器請求狀態、控制台和指令碼錯誤。
- `final.png`、`trace.zip`：結束/失敗時的截圖，以及可回放的操作追蹤。
- `media-server.jsonl`：本地分片伺服器的耗時、位元組數與取消情況。

可用 `pnpm exec playwright-core show-trace <trace.zip路徑>` 開啟追蹤。輸出目錄已經加入 gitignore；檔案儲存在執行命令的伺服器，不會自動上傳。

預設檢查五個場景：

| 場景 | 檢查內容 |
| --- | --- |
| `seek` | 共用播放器的真實滑鼠點選/拖動、拖動鬆手後僅定位一次、首幀恢復、暫停/鍵盤定位、失活取消拖動、開關關閉和儲存失敗 |
| `legacy` | 吃瓜詳情獨立播放器的相同檢查 |
| `stall` | 人為停止 HLS 緩衝後點擊未緩衝位置，檢查 8 秒超時日誌以及恢復緩衝後的首幀；屬於故障注入 |
| `lifecycle` | 兩個播放器延遲清單後失活、後臺禁止自動播放和分片載入、重新啟用後恢復、手動暫停狀態保留，以及影片延遲掛載時保持失活 |
| `mms` | 兩個播放器在兩種 API 同時存在時優先使用標準 MSE；在模擬 MMS-only 環境中，endstreaming 停止緩衝後點擊未緩衝位置，即使沒有 startstreaming 事件也能自動恢復並輸出首幀 |

任何檢查失敗，命令退出碼為 1，仍保留日誌和報告。不要把日誌採集通過理解成所有播放問題已修復。只執行診斷迴歸可以選擇：

```sh
pnpm test:playback --scenarios seek,legacy,stall
```

僅驗證 hls.js 的 MMS 相容修復可執行 `pnpm test:playback --scenarios mms`。該場景由 Chromium 的真實 MSE 完成解碼和緩衝，僅模擬 MMS 建構子與 streaming 事件；報告標記 `simulated_mms: true`，不能替代 macOS 27 實機確認。

其他引數：`--media-dir DIR` 複用已有 `list.m3u8` 與 `seg*.ts`；`--browser-executable PATH` 使用已安裝的匹配版本 Chromium；`--output NEW_DIR` 指定新的產物目錄。`FFMPEG_PATH` 可指定 FFmpeg 執行檔。命令拒絕覆蓋已經存在的輸出目錄。

倉庫附帶手動觸發的 GitHub Actions 工作流 **Playback diagnostics**。它會在檢查失敗時仍上傳日誌、截圖與追蹤，保留 7 天；不會隨推送自動執行。

**覆蓋邊界：** 這些測試使用真實 React 控制元件、video、hls.js 與本地 HTTP 分片，但業務 API 和 Tauri 日誌 IPC 使用替身。它們不執行真實 Rust 媒體代理，不驗證真實 CDN、代理/TUN、Tauri 的 CSP/Origin/DNS 策略，也不能證明 macOS 27 的 ManagedMediaSource 問題已復現或修復。Rust 日誌持久化/輪轉/脫敏仍由本地 Rust 迴歸檢查；macOS 特有問題使用實機 `playback.log` 確認。
