# 任務追蹤：本機 Markdown

沿用本機任務方式；在維護者明確指定 GitHub Issues 前，任務放在本機 .scratch/，該目錄不提交。

- 規格：.scratch/<feature>/spec.md。
- 任務：.scratch/<feature>/issues/<NN>-<slug>.md。
- 任務開頭以 Status: 記錄分類，見 [分類設定](triage-labels.md)。
- 進行中使用 claimed，完成使用 resolved；討論附加於 Comments，保留歷程。
- 有阻礙的相依任務完成後才開始下游工作。
