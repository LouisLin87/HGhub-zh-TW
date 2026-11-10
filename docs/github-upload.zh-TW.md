# GitHub 上傳與發佈

乾淨 ZIP 已整理為原始碼上傳版，沒有 Git 遠端設定，也不包含開發工具、相依套件快取、金鑰、日誌及安裝產物。雙擊啟動後會在本機產生工具與快取，這些項目已由 .gitignore 排除。使用過的資料夾建議透過 GitHub Desktop 提交；網頁上傳請使用乾淨 ZIP 解壓後的內容。

## 建議使用 Fork 與 GitHub Desktop

1. 到 [Minireel/HGhub](https://github.com/Minireel/HGhub) 建立自己的 Fork，例如命名為 HGhub-zh-TW。
2. 使用 GitHub Desktop 將自己的 Fork 複製到新的本機目錄。
3. 將本上傳版的內容同步到該目錄根目錄，保留 GitHub Desktop 建立的 `.git/`。內容應直接呈現 README.md、src/、src-tauri/ 等，避免多包一層資料夾。
4. **同步 `.github/workflows/` 的修改**，確認原有工作流程被本上傳版的同名檔案更新，避免保留原本的自動發佈設定。
5. 在 GitHub Desktop 的變更清單檢查新增、修改及刪除，再提交並推送到自己的 Fork。
6. 在 GitHub 確認 README 的來源連結、LICENSE、NOTICE.md 與檔案結構正確。

若使用全新的儲存庫，上傳這個資料夾的內容即可；來源關係以 README 與 NOTICE.md 記錄。Fork 則另有 GitHub 自動顯示的來源關係。

## 使用網頁上傳時

選取解壓後的原始碼內容，包含 `.github/`、`.gitignore`、`.gitattributes`、`.npmrc` 等隱藏項目。上傳前檢查根目錄層級與完整性；大量檔案建議使用 GitHub Desktop。

ZIP 是搬移原始碼用的封裝。僅把 ZIP 當成一個檔案上傳，GitHub 不會自動展開成專案原始碼。

## 發佈安裝附件

到你自己的儲存庫選擇 Releases，建立草稿，填寫區別於上游的標籤，例如 `v1.2.0-zh-tw.1`，附上：

- 該次 Windows 安裝 EXE 或 Android APK。
- SHA-256 校驗檔。
- 來源、授權、修改摘要及已驗證範圍。

檢查後再由維護者發佈。安裝檔的版本須與說明一致；未實機驗證的平台應保留其測試標示。

## 工作流程的調整

上傳版的五個工作流程沿用原檔名，改為 `workflow_dispatch` 手動觸發，內容權限為讀取，並移除 GitHub Release 發佈工作或步驟。建置產物保存在 Actions 附件。

原工作流程原文另存於 `docs/upstream/workflows/`，該目錄不會作為 GitHub Actions 執行入口。工作流程尚未在你的儲存庫驗證，執行前需確認各平台工具、簽章及範例設定。

## 保留原作者聲明

README.upstream.md 與 LICENSE 維持原文；README.md、NOTICE.md 及修改紀錄清楚標示來源與個人維護內容。散布時遵守 PolyForm Noncommercial 1.0.0，保留 Required Notice。

## 上傳前的最後檢查

- `.tools/`、`node_modules/`、`target/`、`gen/`、`dist/` 及測試產物未提交。
- `.env`、簽章金鑰、帳號設定、私人下載連結與日誌未提交。
- 安裝附件只放到準備發佈的 Releases，來源原始碼保留 LICENSE 與聲明。
- 新增環境設定時，由範例建立本機私有檔案，再確認 `.gitignore` 排除它。

官方操作：[建立 Fork](https://docs.github.com/en/pull-requests/how-tos/work-with-forks/fork-a-repo)、[管理 Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)。
