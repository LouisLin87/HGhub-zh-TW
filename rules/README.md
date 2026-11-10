# HGhub 分流規則

統計日期：2026-11-07。包含 5 條域名字尾規則和 3 條精確域名規則；沒有已確認的固定服務端 IP/CIDR。規則只決定流量如何分流，不改變應用的賬號信任或正文清洗策略。

## 域名來源

| 匹配方式 | 域名 / 字尾 | 用途及證據 |
| --- | --- | --- |
| 字尾 | `huangguoai.com` | 固定 API 線路 1，正文相對 URL 的基址；`src-tauri/src/net.rs`、`src/html.ts` |
| 字尾 | `huangguoai.ai` | 固定 API 線路 2；`src-tauri/src/lines.rs` |
| 精確 | `huangguoai.pages.dev` | 映象釋出頁及其 `publish.js`；`src-tauri/src/lines.rs` |
| 字尾 | `drleao.cn` | 圖片 CDN 樣本 `pic.drleao.cn`；`src-tauri/src/media.rs` 聯網測試 |
| 字尾 | `hzjvlo.cn` | 影片 CDN 樣本 `yd-lphu-hls.hzjvlo.cn`；`src-tauri/src/media.rs` 聯網測試 |
| 字尾 | `xxbgwypx.cc` | 映象樣本 `thu.xxbgwypx.cc` 及動態子域；`src-tauri/src/lines.rs` 測試 |
| 精確 | `api.github.com` | GitHub Release 更新後設資料；`src-tauri/src/api.rs` |
| 精確 | `github.com` | 更新頁面，使用者點選後交給外部瀏覽器；`src-tauri/src/api.rs` |

CDN 和映象三條字尾來自倉庫已有樣本，不能據此認定它們當前仍在使用。當前釋出頁線上核對返回 HTTP 403。API 返回的封面、播放地址，HLS 子清單、分片、金鑰、重定向地址及映象列表都可能引入新域名，因此這是一份原始碼已知目標清單，不是完整流量白名單。新域名應依據代理連線日誌繼續補充。

GitHub 是共享基礎設施，這兩條精確規則也會匹配其他程式對同域名的訪問。應用只打開更新網頁，不自動下載更新包；外部瀏覽器下載時使用的其他 GitHub/CDN 域名未納入本清單。

## Mihomo

使用 [hghub.list](hghub.list)，這是 `classical` 行為、`text` 格式的規則集合。將 `rules` 目錄放在 Mihomo 配置目錄中，併合並以下配置；把 `PROXY` 替換成已有代理組名。規則應放在其他兜底規則之前。

```yaml
rule-providers:
  hghub:
    type: file
    behavior: classical
    format: text
    path: ./rules/hghub.list

rules:
  - RULE-SET,hghub,PROXY
```

也可通過 HTTP 託管該檔案後使用 `type: http`，填寫實際 `url` 和更新週期。`.list` 不包含策略名，不要直接作為完整的 `rules:` 配置貼上。

## sing-box

使用 [hghub.json](hghub.json)，源規則集版本為 2，適用於 sing-box 1.10 及以上。將目錄放在配置目錄中，合併到現有 `route`；把 `proxy` 替換成實際出站標籤，放在兜底規則之前。

```json
{
  "route": {
    "rule_set": [
      {
        "type": "local",
        "tag": "hghub",
        "format": "source",
        "path": "./rules/hghub.json"
      }
    ],
    "rules": [
      {
        "rule_set": ["hghub"],
        "action": "route",
        "outbound": "proxy"
      }
    ]
  }
}
```

以上路由動作寫法適用於 sing-box 1.11 及以上；1.10 使用相同源規則集時，刪除路由項中的 `"action": "route"`。

需要二進位制規則集時可使用已編譯的 [hghub.srs](hghub.srs)，將配置的 `format` 改為 `binary`、`path` 改為 `./rules/hghub.srs`。修改 JSON 後重新編譯：

```sh
sing-box rule-set compile --output rules/hghub.srs rules/hghub.json
```

## IP/CIDR 與本地協議

`198.18.0.0/15` 是應用為相容代理而允許的 FakeIP 地址池，不是上游服務專用網段。不能用它替代域名規則，也不要把共享 CDN 的 DNS 解析結果當作固定專用 CIDR。

`hg://localhost`、`hg.localhost`、`ipc.localhost`、`tauri.localhost` 是 WebView 內部協議；`localhost:1420` 是開發伺服器，`127.0.0.1:7897` 是代理配置示例。它們均未加入遠端代理規則。安全測試中的內網地址、示例域名，以及依賴下載、配置 schema、文件連結也未納入。

如果代理日誌只顯示 IP，請先核對域名嗅探、FakeIP 對映恢復或應用使用系統 HTTP 代理的配置；無法恢復域名時，這份域名規則可能無法匹配。

格式參考：[Mihomo 規則集合](https://wiki.metacubex.one/config/rule-providers/)、[sing-box 源規則集](https://sing-box.sagernet.org/configuration/rule-set/source-format/)、[sing-box 路由遷移說明](https://sing-box.sagernet.org/migration/)。

## 本地驗證

已使用 Mihomo v1.19.32 的 `-t` 和 sing-box v1.14.3 的 `check` 檢查匯入配置；確認 `.list` 與 `.json` 的 8 條目標一致，並對 sing-box 源規則及編譯後的 `.srs` 完成 42 項正反匹配檢查。驗證臨時目錄已自動清理。
