# 客戶端內容與網路安全

## 本次加固的行為

- 文章正文通過固定版本 DOMPurify 清洗，並且將 `DOMPurify.sanitize` 作為進入 `dangerouslySetInnerHTML` 前的最終步驟（冪等處理代理圖片，避免二次解析差異）。嚴格模式下只保留必要的 HTML 排版、連結和圖片，SVG、MathML、指令碼、事件屬性、內聯樣式、非 HTTP(S) URL 及本機/內網主機名均不進入頁面；正文容器統一攔截 `<a>` 點選並交由外部開啟或複製，防止 WebView 原地導航或彈出無守衛新視窗。
- 釋出版啟用 CSP：不允許內聯/遠端/data URL 指令碼，不允許 iframe、表單與頁面基址覆蓋。Blob 僅用於媒體和 hls.js Worker，不在 `script-src` 中放行。應用主視窗拒絕跳轉到遠端頁面（開發模式下亦限制 `1420` 埠僅允許本機或顯式配置的 `TAURI_DEV_HOST`），媒體協議僅嚮應用來源返回 CORS 授權。
- 圖片、HLS、分片、線路探測、映象釋出頁及更新後設資料的請求經過統一目標檢查。只允許 HTTP(S)，拒絕 URL 中的使用者名稱/密碼；按主機和埠快取已校驗並固定 DNS 地址的客戶端 30 秒；到期重新檢查全部 DNS 結果，混合公私網答案整體拒絕。GET 最多跟隨 5 次重定向，每一跳重新檢查，禁止 HTTPS 降級到 HTTP。HLS 的子清單、備用音軌、金鑰及低延遲分片 URI 也經媒體代理改寫，不能繞過目標檢查。停用可從外部 JSON 派生新地址的可選插播/內容引導功能。
- 業務線路必須是純 HTTPS origin，不接受路徑、查詢串、片段或認證資訊。密碼登入和註冊僅允許 `https://huangguoai.com`、`https://huangguoai.ai` 兩個固定站點；遠端映象列表不能擴大賬號信任範圍。映象保留遊客瀏覽。
- token 與具體站點 origin 繫結，跨站點切換清除登入狀態。在途請求捕獲自己的站點快照，不能把舊 token 帶到新站點。包含賬號資訊的業務 POST 不跟隨任何重定向，避免 307/308 重發密碼。舊版會話沒有站點歸屬，升級後需要重新登入。
- 網路診斷只返回代理協議、主機和埠；隱藏代理使用者名稱、密碼、路徑、查詢串、片段及 NO_PROXY 列表。無效代理配置隱藏整個原始值。脫敏發生在 Rust 返回資料之前，因此後續寫入日誌和複製剪貼簿均得到脫敏結果。此前已生成的日誌/已複製的診斷內容不會自動重寫。

## 嚴格安全模式與相容回退（命令列 / 設定開關）

預設開啟**嚴格安全模式**。若上游正文排版、特殊內網環境或自定義 FakeIP 網段因嚴格規則導致功能受阻，可通過以下任一方式回退到相容模式（關閉激進標籤/內網/導航守衛，仍保留基礎 DOMPurify 指令碼與事件清洗及應用內部協議隔離）：

1. **命令列引數**：啟動時傳入 `--no-strict`（或 `--no-strict-security`、`--relaxed`），也可設定環境變數 `HGHUB_STRICT=0`；傳入 `--strict` 可顯式強制開啟。
2. **設定頁開關**：在應用「設定 → 網路 → 嚴格安全模式」中切換，立即生效並持久化到應用資料目錄 `security.json`。

## FakeIP 與內網策略

明確放行 `198.18.0.0/15`，即 `198.18.0.0` 至 `198.19.255.255`，相容常見代理軟體的 FakeIP。此例外同樣適用於 DNS 結果、重定向及 IPv4 對映 IPv6 地址。

拒絕普通私網、迴環、鏈路本地、共享地址空間、組播和保留地址，包括 `10/8`、`172.16/12`、`192.168/16`、`127/8`、`169.254/16`、`100.64/10`、IPv6 ULA/鏈路本地/迴環，以及可封裝私網 IPv4 的 NAT64、6to4、Teredo 地址。拒絕 localhost/.localhost、.local、.internal、home.arpa 主機名。其他自定義 FakeIP 網段沒有自動放行。

已知相容限制（P2-3）：嚴格模式拒絕 NAT64/DNS64 合成地址（包括 `64:ff9b::/96`），且混合公私網答案整體拒絕。因此 IPv6-only 運營商網路若依賴 DNS64，直連媒體或 API 可能無法使用。可使用同時提供正常公網 DNS 解析的可信代理環境，或在瞭解風險後通過設定關閉嚴格模式 / 使用 `--no-strict` 回退；這不會自動放行其他自定義 FakeIP 網段。

系統代理和 `HGHUB_PROXY` 是使用者配置的可信傳輸層，仍可連線 `127.0.0.1:7897` 等本地代理端點；這不意味著媒體 URL 可以指向該地址。直接連線會使用檢查後固定的 DNS 結果；HTTP(S) 代理可能自行解析域名，FakeIP/TUN 也會重新對映目標，應用無法獨立驗證代理/TUN 最終實際連線的地址。需要同時信任並配置好代理軟體；本策略不替代代理側內網訪問控制。

## 本地迴歸驗證

```sh
pnpm build
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

以上 Rust 測試預設跳過 5 項 `#[ignore]` 聯網測試（`media::tests::live_play_chain`、`media::tests::cover_decrypt_works`、`lines::tests::mirrors_live`、`lines::tests::probe_live`、`net::tests::diag`），結果為 28 passed / 0 failed / 5 ignored，覆蓋普通內網/FakeIP 邊界、IPv4 變體、IPv6、混合 DNS 答案、重定向、憑據站點繫結、舊會話和代理脫敏，不傳送真實賬號資訊。

需要真聯網時顯式跑：`cargo test --locked --lib -- --ignored`。其中封面測試要求當次有效的地址，用 `HGHUB_TEST_COVER_URL` 傳入；播放鏈路與線路探測依賴上游即時介面。迴歸測試不再硬編碼帶 `auth_key` 時效簽名的 URL——舊寫法的簽名已於 2026-11-03 過期，會讓 `cargo test --lib` 穩定失敗。

瀏覽器迴歸入口為 `tests/html-security.html`。啟動 `pnpm dev --host 127.0.0.1` 後，在瀏覽器開啟 `http://127.0.0.1:1420/tests/html-security.html`；結果應為 `passed: 31, failed: 0`。測試使用無害頁面變數，覆蓋嚴格及相容模式的原 SVG 漏洞、混淆 URL、事件屬性、名稱空間/HTML 變異樣本，以及正常排版、懶載入圖片、重複清洗和安全連結。

本次驗證包括 macOS 上的前端構建、Rust 編譯和本地測試，以及獨立 Chromium 的 15 項正文迴歸與 5 項生產 CSP 檢查；沒有啟動真實 Tauri 應用連線上游，也沒有完成 Windows/Android 實機播放和真實 FakeIP/TUN 聯調。

2026-11-07 本地複測：Linux 上前端生產構建、迴歸測試 TypeScript 檢查及 `cargo check --locked` 通過；Rust 本地測試 16 項通過、0 失敗（跳過上述 6 項聯網測試）。獨立 Chromium 分別模擬 Windows、macOS 使用者代理，驗證 `http://hg.localhost` 與 `hg://localhost` 兩套圖片協議分支，每套 29 項全部通過；這不等同於原生 Windows/macOS WebView 實測。複測修復了最終 URL 規則誤刪連結 `target`/`rel` 和 macOS 代理圖片重複清洗丟失的問題。Linux 編譯庫僅下載、解包到 `/tmp` 使用，未安裝到系統。

2026-11-07 原生補測：系統 GTK/WebKit 依賴安裝完成後，構建嵌入生產前端資源的 Tauri 2 除錯程式，在 Linux WebKitGTK 2.52.6 中通過 WebDriver 驗證。使用獨立資料目錄，12 次啟動均完整關閉原生程序後再啟動，116 項檢查通過，覆蓋預設啟動靜音、手動開聲後連續重啟、關閉/重新開啟偏好、音量記憶、操作後立即關閉，以及普通影片和文章影片共享靜音狀態。媒體使用本地 WAV 替身，業務內容介面使用測試資料；設定狀態、WebKit 磁碟儲存和視窗關閉 IPC 均來自真實客戶端。已有正文安全迴歸在原生 WebView 中 29 項全部通過；這仍不等同於 Windows/macOS/Android 實機或真實上游播放鏈路驗證。

同次重跑全部 Rust 測試，未跳過任何專案：19 項通過、3 項失敗。`mirrors_live` 與 `live_play_chain` 因當前 DNS 將目標解析為普通私網地址而被嚴格策略拒絕；`playlist_rewrite_works` 使用的舊地址返回 HTTP 403。此前跳過的封面解密、線路探測和網路診斷測試完成執行，其中探測/診斷測試通過僅代表返回結果可生成，不代表上游可達。未因此放寬內網訪問限制。

審查後續修正與未採用建議的逐項說明、對應 commit 及驗證範圍，見 [安全審查後續複核](review-followup.md)。本輪前端生產構建（含測試型別檢查）、Rust 編譯檢查通過；本地 Rust 迴歸 28 項通過（5 項聯網測試標記 `#[ignore]`），瀏覽器迴歸 70 項通過。安全模式現在僅由 Rust 的 `security.json` 持久化，前端收到後端確認後才切換；診斷 DNS 僅報告數量和 FakeIP 標記。匿名業務 GET 可逐跳校驗後跟隨 301/302/303，帶憑據/請求體的業務請求不跟隨；只有無請求體的邏輯 GET 允許重試。HLS 時間線後設資料保留，外部 URI 和資源列表欄位仍過濾。

本次僅處理審查編號 1、3、5、8；媒體/日誌容量上限和 token 明文儲存仍需另行處理。外鏈改用固定版本 `tauri-plugin-opener` 2.7.2（鎖定 `open` 5.4.5），HTTP(S) URL 仍經 Rust 地址策略校驗，以預設處理器 `with = None` 開啟。Windows 路徑啟用 `shellexecute-on-windows`，通過 `ShellExecuteExW` 傳遞完整 URL，不經過命令直譯器；關閉外掛自動接管連結，且不給前端授予 `opener:*` capability，繼續統一經過現有校驗入口（P1-1）。外掛要求 Rust 1.90，專案最低版本宣告同步調整。網路錯誤文本的 UTF-8 截斷在改動網路入口時一併改為安全截斷。

參考：[DOMPurify](https://github.com/cure53/DOMPurify)、[Tauri CSP](https://v2.tauri.app/security/csp/)、[reqwest DNS 地址固定介面](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html#method.resolve_to_addrs)。
