# HGhub 遊客播放相容性修正

日期：2026-11-10

## 症狀與原因

使用者回報未登入時有封面，但播放時間停在 `00:00`。程式碼確認未登入即為遊客，無需建立帳號。

舊程式以 GET `/api/videos/:id/play` 查詢 `video_url`。實際匿名測試發現，舊入口回傳成功狀態，但播放地址為空。上游目前公開的播放程式碼改為加密通道，使用 POST `/api/videos/:id/episodes/:ep/playback`，從 `streams.main` 取得來源，並可能附帶 CDN 所需的請求標頭。

此差異已由一個匿名重現測試確認：修正前因沒有來源地址而失敗；修正後可取得地址，且本機代理能讀取有效的 HLS 文字清單。這項測試沒有下載影音分片。

## 本次變更

- Rust 播放查詢對接現行加密通道，使用既有依賴樹中的固定版本 `ring 0.17.14` 實作 P-256／HKDF-SHA256／AES-256-GCM。每次請求使用新的握手及一次性的回應物件；錯誤 session、請求序號、時間或驗證標籤均拒絕。
- 保留既有前端 `video_url` 契約，在後端將 `streams.main` 轉換為播放器可用資料。
- 每次播放使用獨立 `source_id`，CDN 標頭只留在有容量上限的程式記憶體，不寫入日誌或傳給前端。媒體要求固定使用原線路、來源與授權快照；跨來源重新導向不會帶出該授權。
- 主／子清單、金鑰及分片持續攜帶來源識別；清單與媒體快取也依來源分隔，避免同一 CDN 上的不同影片互相覆蓋。
- 返回同一影片或同一集時重新取得短效播放來源；頁面不在活動狀態時不發起播放取址。
- 自檢正確分開解析上游 URL 與來源識別，避免重複包裝與遺失識別；移除會記錄部分播放直鏈的舊診斷輸出。

其他業務介面沿用原本路徑；原有 DNS／內網／重新導向檢查繼續套用。

## 驗證結果

| 專案 | 結果 |
| --- | --- |
| 舊入口匿名重現 | 修正前穩定失敗：沒有播放地址 |
| Rust 本機回歸 | 47 通過、0 失敗；6 個聯網測試保留手動執行標記 |
| 匿名播放介面與 HLS 代理 | 個別執行通過，清單有效且子資源保留來源識別；無影音分片下載 |
| 前端播放來源回歸 | 5 項通過，所有外部請求均封鎖 |
| 既有 API 載入回歸 | 首頁錯誤／重試／空結果、分類獨立載入與重試通過 |
| 型別檢查與前端建置 | 通過；仍有既有的大型 JS chunk 提示 |
| Windows 桌面開發執行檔 | 已重新建置成功，未自動啟動真實內容播放 |
| 使用者實際播放重測 | 2026-11-10 回報「完成，可順利播放」；本次問題已確認恢復 |
| 加密條件突變檢查 | 在獨立副本移除 session ID 檢查後，回歸測試如預期失敗；正式檔未弱化 |

加密向量由獨立撰寫的 Node crypto 與 WebCrypto 交叉核對，資料全部為人工測試值。程式審查指出的跨線路快照、同來源授權覆寫、返回頁面重新整理與自檢 URL 解析問題，均已修正並加入驗證。

後續實際重測：2026-11-10，使用者回報「完成，可順利播放」，確認本次問題已恢復。此結果涵蓋使用者當次測試，不延伸為所有影片、所有線路或長時間播放的驗證。

## 使用方式

完全關閉舊的 HGhub 與開發命令視窗，再雙擊專案根目錄 `hghub.cmd`，或執行：

```powershell
Set-Location -LiteralPath 'C:\Dev\HGhub-zh-TW'
.\hghub.cmd app:dev
```

保持「我的」頁面為未登入，直接測試首頁或分集播放，確認播放時間能從 `00:00` 往前走。

本機回歸入口：

```powershell
$env:BROWSER_EXECUTABLE = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
.\hghub.cmd test:playback-source
.\hghub.cmd test:api-loading
.\hghub.cmd exec cargo test --locked --offline --manifest-path src-tauri/Cargo.toml --lib
```

只檢查匿名取址與文字清單的聯網測試：

```powershell
.\hghub.cmd exec cargo test --locked --manifest-path src-tauri/Cargo.toml --lib guest_playback_metadata_has_source -- --ignored
```

## 協定來源

本次靜態檢查上游公開的 `edge-api-crypto.js`、`edge-cache-runtime.js`、`services/playback.js` 及 `pages/cyber-video.js`，版本引數為 `20261109001-1`。未執行下載的遠端指令碼。

加密 API 使用方式另參考 [ring 的 ECDH 檔案](https://docs.rs/ring/0.17.14/ring/agreement/index.html)、[HKDF 檔案](https://docs.rs/ring/0.17.14/ring/hkdf/index.html) 及 [AEAD 檔案](https://docs.rs/ring/0.17.14/ring/aead/index.html)。
