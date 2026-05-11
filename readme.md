# codonx 0.0.3 MVP 边界

当前主文档见 [README.md](README.md)。本文件保留小写文件名，方便按
`readme.md` 查找。

0.0.3 的目标仍然是小而可靠：

- 支持 `#%ifdebug`、`#%else`、`#%endif` 文本预处理。
- 支持正则/行级别能安全完成的 Python 调试目标语法降级。
- 支持基础 `assert`/guard，用来尽早发现明显的 Codon/Python 行为差异。
- 支持预处理后透传 `codon run` / `codon build`。
- 支持 `#%define CODON_PYTHON` 和 `#%define CODON_DEBUG`，用于给 Codon
  子进程注入环境变量和生成 debug dump。

## 两条独立边界

语法降级和语义 guard 必须分开判断：

- 语法层面只回答：能不能把 Codon 源码机械改成 Python 3.12 可解析源码。
- 语义层面只回答：能不能用 Python 动态检查高效暴露明显不一致。

能自动化的只做白名单。超过正则/行级解析能力的内容，默认由开发者用
`#%ifdebug / #%else / #%endif` 显式维护，并让分叉点尽量小。

## 0.0.3 自动语法降级白名单

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

## 0.0.3 `#%define` 白名单

`#%define` 是 codonx 自己消费的指令，不进入 Python 或 Codon 输出文件。

支持：

- `#%define CODON_PYTHON /path/to/libpython3.12.so`
- `#%define CODON_DEBUG target/codon_debug`

行为：

- `CODON_PYTHON` 会作为环境变量注入 `codon run` / `codon build` 子进程。
- `CODON_DEBUG` 也会作为环境变量注入；相对路径按运行 `codonx` 时的当前工作
  目录解析。
- 如果 `CODON_DEBUG` 已定义且 Codon 调用处于 debug 模式，即默认模式或显式
  `-debug` / `--debug`，codonx 会创建该目录，把 Codon 子进程工作目录切到该
  目录，并自动追加 `-log l`，除非用户已经传了 `-log`。
- 如果显式传了 `-release` / `--release`，只注入环境变量，不自动追加
  `-log l`，也不切换工作目录。

实测 Codon `-log l` 会在当前工作目录生成 `_dump_typecheck.sexp`、
`_dump_typecheck.htm`、`_dump_ir.sexp`、`_dump_ir_opt.sexp`、`_dump_llvm.ll`
等 dump 文件。

未知 `#%define` 名称会报错。0.0.3 不提供通用宏系统。

## 0.0.3 guard 白名单

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

以下内容 0.0.3 不自动处理：

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
