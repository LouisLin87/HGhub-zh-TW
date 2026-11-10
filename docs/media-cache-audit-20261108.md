# 封面與播放列表快取對比

對比基準：Minireel/HGhub `8665ffcaa553bc6eb1b52629fd91b35cc6e02678`。檢查物件為 2026-11-08 當前工作區程式碼，包括尚未提交的修改。基準提交已在本地倉庫中，使用 `git show`/`git diff` 比對，未訪問上游介面。首次核查發現子播放列表快取指令被收緊；後續按保持上游行為的要求，已將該成功響應恢復為 1 小時快取。

## 內建記憶體快取

| 專案 | 基準提交 | 當前程式碼 | 結論 |
| --- | --- | --- | --- |
| 封面 `IMG_CACHE_MAX` | 240 條 | 240 條 | 未縮小、未關閉 |
| 主播放列表 `PL_CACHE_MAX` | 60 條 | 60 條 | 未縮小、未關閉 |
| 快取鍵 | 完整 URL | 完整 URL | 相同，簽名/查詢引數改變會產生不同條目 |
| 命中行為 | 下載前查記憶體，命中直接返回 | 相同 | 保留 |
| 寫入行為 | 下載處理後寫入 | 相同 | 保留；播放列表新增格式校驗，錯誤頁不入快取 |
| 淘汰行為 | 達到上限後移除 HashMap 中一個條目 | 相同 | 未改成更小容量或更短 TTL |
| 清空入口 | `cache_clear` → `media::clear_caches()` | 相同 | 沒有新增自動清空呼叫 |

`cache_put`、`load_image`、`clear_caches` 函式去除空白後與基準一致。`load_playlist` 仍在請求網路之前讀取 `PL_CACHE`，成功改寫後仍呼叫 `cache_put(..., PL_CACHE_MAX)`。兩項快取均為程序記憶體，重啟後丟失，與基準一致；容量單位是條目數，不是 MiB。

網路層新增的 30 秒 `CLIENT_TTL` 用於 HTTP 客戶端/DNS 檢查結果複用，不是封面或播放列表內容快取的有效期。

## HTTP 快取指令

| 資源路徑及實際內容 | 基準提交 `Cache-Control` | 當前程式碼 | 結論 |
| --- | --- | --- | --- |
| `/img` 封面成功響應 | `public, max-age=86400` | 相同 | 未改變 |
| `/hls` 主播放列表成功響應 | `no-store` | 相同 | 原本就如此，不影響 Rust 的 60 條記憶體快取 |
| `/seg` 影片分片/金鑰 | `public, max-age=3600` | 相同 | 未改變 |
| `/seg` 內的子播放列表 | 按普通分片處理，`public, max-age=3600` | 識別後改寫內部 URI，`public, max-age=3600` | 快取指令已恢復一致，保留 URI 改寫 |

最後一項在當前 `media::segment_response` 中按響應型別或 `#EXTM3U` 內容識別。子播放列表走 `fetch_bytes` 後直接生成響應，不使用 `PL_CACHE`；基準提交的這條 `/seg` 路徑也不使用 `PL_CACHE`。此前該成功響應曾使用 `no-store`，但主播放列表的 60 條記憶體快取始終沒有因此關閉。

子播放列表的變化伴隨巢狀播放列表 URI 改寫，確保內層媒體地址繼續經過本地代理。URI 改寫和 HTTP 快取時效相互獨立，沒有現有功能要求將成功響應設為 `no-store`，所以已恢復上游的 `public, max-age=3600`。解碼失敗及錯誤響應仍使用 `no-store`，原有 URI 改寫、內容型別識別和安全檢查保留。實際 WebView 是否複用自定義協議響應，仍需結合核心行為和實機請求記錄判斷。

## 檢查位置

- `src-tauri/src/media.rs`：容量常量、讀寫/淘汰邏輯、`load_image`、`load_playlist`、`segment_response`、`handle`。
- `src-tauri/src/api.rs`：`cache_clear` 命令；前後都只有這一處生產程式碼呼叫 `clear_caches`。
- `src/api.ts`、設定頁：本地媒體協議對映及使用者主動清理快取入口。
