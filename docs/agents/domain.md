# 領域檔案

本專案採 single-context：根目錄 `GLOSSARY.md` 與 `docs/adr/`。`pnpm-workspace.yaml` 目前只是安裝設定，不代表具有多個套件的 monorepo。

## 使用方式

- 探索術語前，若有根目錄 `GLOSSARY.md`，先讀它；若後續建立 `GLOSSARY-MAP.md`，改讀其中與任務相關的領域詞彙表。
- 探索架構前，讀 `docs/adr/` 中與任務相關的決策；若後續改為多領域，也檢查 `src/<context>/docs/adr/`。
- 上述檔案不存在時直接繼續，不為初始化建立空白詞彙表或虛構 ADR；待實際釐清術語或做出架構決策時再建立。
- Issue、規格與程式中的領域名稱沿用詞彙表；若出現新概念，先確認是否確有檔案缺漏。
- 方案若與既有 ADR 衝突，明確標出衝突的 ADR 與需要重議的理由，不直接覆寫既有決策。
