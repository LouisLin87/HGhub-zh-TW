# 桌面進度條點選誤暫停迴歸

症狀：順序播放和拖動進度條正常，控制條閒置隱藏後直接點選靠後位置，影片不跳轉或變成暫停，按鈕顯示“播放”。

原因：隱藏狀態給整個控制條設定了 `pointer-events: none`。滑鼠移動喚醒控制條的 React 更新尚未提交時，緊接的點選可能落到下方影片，觸發點選暫停；拖動會給喚醒過程留出更多時間。

修復：閒置隱藏時僅保留進度條的命中區域；非啟用播放頁停用進度條。其他隱藏按鈕仍不接收點選。

## 本地複測

需要 FFmpeg、Python 3 和專案開發依賴。使用本地生成的 90 秒 HLS，避免依賴上游。

```sh
seek_fixture_dir=$(mktemp -d)
ffmpeg -hide_banner -loglevel error \
  -f lavfi -i testsrc2=size=320x180:rate=15 \
  -f lavfi -i sine=frequency=440:sample_rate=44100 -t 90 \
  -c:v libx264 -preset ultrafast -g 30 -sc_threshold 0 -pix_fmt yuv420p \
  -c:a aac -f hls -hls_time 2 -hls_playlist_type vod \
  -hls_segment_filename "$seek_fixture_dir/seg%03d.ts" "$seek_fixture_dir/list.m3u8"
python3 tests/serve-seek-media.py "$seek_fixture_dir"
```

另一個終端執行 `pnpm dev --host 127.0.0.1`，開啟 `http://127.0.0.1:1420/tests/player-seek.html`。測試頁使用真實 Feed、VideoSurface、PlayerControls 與 hls.js，僅業務資料和媒體協議傳輸使用本地替身。頁面會自動檢查閒置時進度條是否仍是點選命中目標，結果寫入 `#results`。

檢查以下操作：

- 等待控制條隱藏，直接點選 80%、20%、65% 位置；應跳到目標附近並持續播放。
- 拖動進度條仍可定位並繼續播放。
- 暫停後點擊定位，應保持暫停；聚焦進度條後方向鍵仍有效。
- 點選“切換播放頁啟用狀態”，失活後進度條應停用。

本輪 Chromium 與 Linux WebKitGTK 各通過上述 12 項自動化斷言。修改前 Chromium 復現了點選後暫停且未 seek；WebKitGTK 未跳轉到目標。修改後均可跳轉並持續播放。WebKitGTK 驗證不能代替 macOS WKWebView 實機確認。

## 點選與拖動提交

桌面控制條和吃瓜詳情獨立播放器共用指標處理：按下時捕獲指標，移動時僅預覽位置與時間，鬆開後實際定位一次。點選按軌道位置計算時間，避免依賴各平臺原生滑塊的點選步進；鍵盤保留原生 range 操作。指標取消、失焦、頁面失活或換源會取消預覽，不誤提交舊位置。原先一次拖動可觸發多次定位，現在無頭回歸會斷言移動期間零次定位、鬆手後一次定位。

完整自動化入口與產物說明見 [播放診斷日誌](playback-diagnostics.md)。
