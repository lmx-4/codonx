# codonx 0.0.2 MVP 边界

当前主文档见 [README.md](README.md)。本文件保留小写文件名，方便按
`readme.md` 查找。

0.0.2 的目标是小而可靠：

- 支持 `#%ifdebug`、`#%else`、`#%endif` 文本预处理。
- 支持正则/行级别能安全完成的 Python 调试目标语法降级。
- 支持基础 `assert`/guard，用来尽早发现明显的 Codon/Python 行为差异。
- 支持预处理后透传 `codon run` / `codon build`。

## 两条独立边界

语法降级和语义 guard 必须分开判断：

- 语法层面只回答：能不能把 Codon 源码机械改成 Python 3.12 可解析源码。
- 语义层面只回答：能不能用 Python 动态检查高效暴露明显不一致。

能自动化的只做白名单。超过正则/行级解析能力的内容，默认由开发者用
`#%ifdebug / #%else / #%endif` 显式维护，并让分叉点尽量小。

## 0.0.2 自动语法降级白名单

- 删除 `@par` / `@par(...)` 装饰器行，保留串行 `for`，并在 Python 输出中
  留下 `codonx:` 注释。
- 删除 `@gpu.kernel`，留下 warning 注释；不模拟 GPU。
- 删除 `@python`，留下 warning 注释；Python debug 直接执行函数体。
- `from python import module` / `from python import module as alias` 改为普通
  `import`。
- `i8/u8/i16/u16/i32/u32/i64/u64` 注解改成 `int`，`f32/f64` 改成
  `float`。

typed Python interop，例如 `from python import mod.fn(int) -> int`，不自动
转成可执行 Python。Python 输出会写注释和 report warning，源码应显式分叉。

## 0.0.2 guard 白名单

默认 `--assert shallow`：

- 检查 `int/i64/u64/i32/u32/i16/u16/i8/u8` 值域。
- 检查 `float/f32/f64`、`bool`、ASCII `str`。
- 检查 `list/set/dict/tuple` 外层形状和 tuple 长度。
- 对函数参数、显式注解赋值、返回值插入 guard。

`--assert full` 递归检查容器元素。未知类型软通过。

这些 guard 只用于提前发现 mismatch，不承诺行为一致。并行数据竞争、GPU、
浮点舍入、Codon overload/generic 分派、Python interop 转换、dict/set 顺序等都
必须显式关注或用 Codon 侧测试覆盖。

## 必须显式维护

以下内容 0.0.2 不自动处理：

- `@tuple`、`@extend`、函数/方法重载。
- Python 3.12 泛型和 Codon `T: type` 之间的语义转换。
- Codon-only `match` 扩展。
- OpenMP、GPU kernel 真实语义、指针、C/LLVM interop。
- 任何需要 AST、类型检查或 Codon 语义知识才能安全转换的结构。

## CLI 摘要

- `codonx --dbg input.codonx` 生成 `input_dbg.py`，默认带 shallow guard。
- `codonx codon input.codonx` 生成 `input_pre.codon`。
- `codonx run ... input.codonx` 预处理后调用 `codon run`，默认删除
  `input_pre.codon`。
- `codonx build ... input.codonx` 预处理后调用 `codon build`。
- `--keep-pre` 保留预处理文件。
- `CODONX_CODON_BIN` 或 `--codon-bin` 可指定 Codon 编译器路径。
