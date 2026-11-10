# 安全審查後續複核

P2、P3 子編號按維護者原文的列舉順序補齊。修正均在 `security-audit`，不包含 `routing-rules` 的啟動靜音或規則變更。

| 編號 | 處理 | Commit | 複核結論 |
| --- | --- | --- | --- |
| P1-1 | 修正 | 54511e0 | 固定使用 tauri-plugin-opener 2.7.2 / open 5.4.5，經現有 Rust HTTP(S)/地址校驗後以 with=None 呼叫預設處理器。Windows 啟用 shellexecute-on-windows，完整 URL 交給 ShellExecuteExW，不回退 cmd。關閉自動接管連結，不授予前端 opener:* 許可權；最低 Rust 調整為 1.90。 |
| P1-2 | 保持 | — | 當前門停用 is_some_and，缺失 Origin 已獲允許；Origin: null 會被拒絕。結合使用者反饋的 macOS 實測，未證實該相容問題，因此未增加 null 的泛化放行。 |
| P1-3 | 保持 | — | 鎖定版本的 tauri-codegen 具有 inject_script_hashes，自動對入口 HTML 的內聯指令碼生成 CSP SHA-256 hash，並將其併入執行時策略。與使用者 macOS 無異常的觀察一致，無需 unsafe-inline，也未移除提前應用主題的指令碼。 |
| P2-1 | 修正 | f3c8df1 | 注意真實傳輸層始終是 POST /api/edge/request 信封。僅無 token、無邏輯 body 的 GET 可跟隨 301/302/303；逐跳檢查 URL、DNS/固定地址及 HTTPS 降級，最多 5 跳。保留信封 POST，不讓客戶端自動改成 GET。307/308、帶憑據或邏輯請求體的請求均拒絕。 |
| P2-2 | 修正 | d3c4094 | 客戶端快取改為按嚴格模式、主機、埠查詢，已驗證的 DNS 地址隨客戶端固定，30 秒 TTL 不因命中延長。到期重新解析並檢查全部結果；模式/代理重新整理會清空快取。 |
| P2-3 | 補充文件 | 58ec4b5 | 明確 NAT64/DNS64、IPv6-only 和混合 DNS 答案的限制，以及相容回退方式。僅配置 HTTP 代理不一定能修復本地 DNS64 解析。 |
| P2-4 | 修正 | efbb664 | 移除 localStorage 安全偏好。Rust security.json 是持久化真源，前端啟動預設嚴格，收到 app_boot 後同步。設定先儲存後更新有效狀態；後端寫入失敗會返回錯誤並保持舊模式。儲存中/尚未初始化時停用設定開關。當前文章隨 context 更新並重新清洗。 |
| P2-5 | 修正 | 19eddf2 | Feed 向面板傳原始正文；ArticlePanel/文章詳情只在正文或安全模式改變時重新清洗。仍保留 sanitizeHtml 函式內部完整的入口清洗、URL 重寫和出口清洗。 |
| P3-1 | 修正 | 6dae653 | HLS 屬性按引號處理逗號，逐個改寫 URI 屬性，絕對化和代理改寫均覆蓋所有欄位。 |
| P3-2 | 修正 | e68c9b8 | 保留 DATERANGE 的時間線、SCTE-35 等非 URL 後設資料；去掉 URI 欄位與 X-ASSET-LIST。CONTENT-STEERING 仍停用，防止衍生未受控請求。 |
| P3-3 | 修正 | 9d5597b | 入口和出口白名單均保留 ol 的 start，並新增編號保留迴歸。 |
| P3-4 | 修正 | 410c2ed | 新增 tsconfig.tests.json，以 ES2022 支援迴歸程式碼的頂層 await；pnpm build 同時檢查 src 與 tests。 |
| P3-5 | 部分修正 | 9622b8c | 補齊前端已知保留 IPv4 網段。審查所說 Rust 漏掉的 2001:2::/48、2001:10::/28 已被 s[1] < 0x0200 覆蓋，新增邊界迴歸而不重複修改 Rust 策略。前端仍是粗篩，後端校驗是請求權威。 |
| P3-6 | 暫不採用 | — | 這是已說明的安全/體驗取捨。兩個固定站點也是不同 origin；當前切站清除單份 session，且不會將舊 token 帶到新站點。按 origin 儲存多份憑據屬於新增賬號管理能力，可後續單獨設計、審查和遷移；本輪不擴大憑據持久化範圍。 |
| P3-7 | 修正 | f4ce4c4 | 診斷 DNS 欄位只保留去重結果數量與是否包含 FakeIP；解析錯誤返回固定摘要，避免解析 IP 列表進入日誌、剪貼簿或對話方塊。 |
| P3-8 | 修正 | c1734fd | 僅邏輯 GET 且無 body 的請求允許重試；登入、註冊和其他變更請求僅嘗試一次。原有 status=0 業務錯誤位於重試塊外，並非所有密碼錯誤本來都會重試；網路/HTTP 錯誤重放風險確實存在。 |

## 驗證

- 前端生產構建通過；包含官方 DOMPurify 型別與 tests 型別檢查。
- Chromium：Windows/macOS 使用者代理下各 31 項正文清洗迴歸通過；嚴格模式初始化、儲存失敗、狀態一致性及已開啟文章重新整理共 8 項通過。
- 外掛方案：Windows 目標的 Cargo feature 樹確認 `open 5.4.5` 啟用了 `shellexecute-on-windows`，未啟用 `insecure`；沒有進行 Windows 原生執行測試。新增 capability 迴歸，防止前端通過外掛命令繞過 URL 校驗。
- `cargo check --locked --offline --lib` 通過；Rust 本地迴歸 **26 passed / 0 failed**，原有 6 個聯網測試 filtered out。覆蓋新增重定向邊界、30 秒快取邊界、儲存失敗保持模式、HLS 多 URI/時間線保留、地址篩選、DNS 脫敏和非冪等操作不重試。

本輪未重跑 Windows/macOS/Android 實機；P1-2、P1-3 的 macOS 使用情況來自使用者反饋。真實上游聯網測試依賴站點與 DNS，本輪按審查者的本地測試範圍排除原有 6 項聯網測試。

參考：[tauri-plugin-opener 2.7.2](https://docs.rs/tauri-plugin-opener/2.7.2/tauri_plugin_opener/)、[open 5.4.5](https://docs.rs/open/5.4.5/open/)、[Tauri CSP](https://v2.tauri.app/security/csp/)。P1-3 還核對了本機鎖定版本 tauri-codegen 2.7.4 的 `src/context.rs::inject_script_hashes`。
