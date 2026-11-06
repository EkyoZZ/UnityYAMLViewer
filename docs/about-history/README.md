# 关于窗口更新历史

每个版本一份 UTF-8 Markdown 文件，例如 `1.3.5.md`。这里保存面向使用者的简短七语言摘要；完整功能、限制与验证证据仍维护在 `docs/versions/`。

新增版本时复制下列格式，补齐七语言内容，并更新 index.json 和 Cargo.toml。无需修改 main.rs 或通用翻译表。索引中的第一版必须与 Cargo.toml 一致。

## 索引控制

index.json 控制版本顺序及展开规则：

~~~json
{
  "initial_visible_versions": 2,
  "older_expanded_on_open": false,
  "versions": ["1.3.5", "1.3.4", "1.3.3"]
}
~~~

- versions 按显示顺序列出全部版本，对应同名 Markdown 文件；程序严格沿用这个顺序，不自行排序。新增版本时在列表首位插入新版本，保留历史条目。
- initial_visible_versions 指定始终显示的前几个版本；older_expanded_on_open 指定每次打开关于窗口时，其余历史是否展开。
- 按当前工程约定，保持 initial_visible_versions 为 2、older_expanded_on_open 为 false，列表按当前版、最近上一版及更早版本排列。
- 索引错误、重复版本、文件缺失、文档未登记、未知设置或翻译不完整都会使编译检查失败，不会静默丢失历史。
- 索引和文档在构建时一起嵌入程序，运行时无需外部文件。

## 版本内容格式

```markdown
# 1.3.5

## zh-CN

- 更新摘要。

## zh-TW

- 更新摘要。

## en

- Update summary.

## ja

- 更新内容。

## ko

- 업데이트 요약.

## de

- Zusammenfassung der Änderungen.

## fr

- Résumé des modifications.
```

- 七种语言小节必须各出现一次，内容非空、条目数量一致；不完整的文件会使编译检查失败，错误中包含版本和原因。
- `- ` 开头的行显示为项目符号；其他非空正文行显示为普通段落，每行一个条目。仅支持此处约定的简单格式，不解析链接、内嵌 HTML、嵌套列表或 Markdown 折叠标签。
- 语言小节用于切换显示内容；当前版和上一版默认显示，其余版本由界面的“查看更多历史”统一折叠，重开关于窗口恢复折叠。
- 文件在构建时嵌入 EXE，运行时不读取磁盘上的历史文档；发行时无需附带这些文件。修改文件后需重新构建才会更新程序中的内容。
- 保留旧版本摘要及原翻译。更新当前版本内容时，七语言同步维护，不把后续功能归入旧版本。
