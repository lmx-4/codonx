# codonx CLI 行为

当前主文档见 [README.md](README.md)。本文件保留小写文件名，方便按 `readme.md` 查找。

核心行为：

- `codonx --dbg input.codonx` 生成 Python 调试文件，默认输出 `input_dbg.py`，默认 `--assert shallow`。
- `codonx codon input.codonx` 只生成纯 Codon 文件，默认输出 `input_pre.codon`。
- `codonx run ... input.codonx [program args...]` 先生成 `input_pre.codon`，再调用 `codon run ... input_pre.codon [program args...]`，结束后默认删除预处理文件。
- `codonx build ... input.codonx` 先生成 `input_pre.codon`，再调用 `codon build`；没有 `-o` 时按原输入文件名补出 Codon 默认输出名，避免生成 `input_pre` 系列产物。
- `--keep-pre` 会保留 `run/build` 使用的预处理文件。
- 默认编译器是 PATH 中的 `codon`；可用 `CODONX_CODON_BIN=/path/to/codon` 或 `--codon-bin /path/to/codon` 覆盖。

## Codon/Python 差异处理原则

后续工作必须把“语法降级”和“语义模拟”分开处理：

- 语法层面只回答：Codon 源码能不能被机器稳定改写为 Python 3.12 可解析源码。能自动化的就做机械转译；不能自动化的必须由开发者用 `#%ifdebug / #%else / #%endif` 显式维护分叉，并让分叉点尽量小。
- 语义层面只回答：生成的 Python 调试目标能不能用动态检查、`assert` 或 helper guard 高效暴露与 Codon 的行为差异。能检查的就自动插入 guard；不能检查的也必须在生成的 Python 文件和报告里加注释/warning，提醒开发者显式关注。
- “语法可转译”不等于“语义等价”；“语义可 guard”也不代表语法不需要分叉。两条线分别建模、分别实现、分别报告。

官方文档依据：

- Codon 自称接近 Python，但不是 CPython 的 drop-in replacement；部分动态特性不支持。
- Codon `int` 是有界 64-bit，字符串当前是 ASCII，dict/set 无序，tuple 长度/部分索引需要编译期已知。
- Python 3.12 官方支持结构化 `match` 语句，支持函数/类/类型别名的类型参数列表，例如 `def f[T](...)`、`class C[T]: ...`、`type A[T] = ...`。这部分不应按旧 Python 版本误判为不可解析语法。
- Python 3.12 的 `match` 是 compound statement，不是表达式。Codon 的 `match` 也是语句形态，但有 Rust-inspired 扩展模式，例如 `case 2 ... 10`。
- Codon 仍有 Python 不支持或语义不同的扩展语法和编译器特性：`@par`、GPU kernel、`from python import`、`@python`、`@tuple`、`@extend`、显式类型实参参数 `T: type`、Codon-only match 范围/省略模式等。

本节核对过的官方资料：

- Python 3.12 compound statements：`match` statement 与 type parameter lists。
- Python 3.12 What's New：PEP 695 type parameter syntax。
- Python 3.12 expressions reference：表达式章节不包含 `match`，因此不要把 `match` 称作表达式。
- Codon language overview/differences：数据类型、静态类型限制、numerics 与 Python 差异。
- Codon generics/functions：`def f[T]`、`class C[T]`、`T: type` 两种泛型写法。
- Codon basics：Codon `match` 扩展，包括 `case 2 ... 10`。

## 语法层面：可机械转译为 Python 3.12

目标：覆盖大约 60%-80% 的 Codon/Python 表层语法差异。只做保守、局部、可审查的改写；遇到不确定结构时不要猜。

优先自动转译：

- `@par(...)` / `@par` 修饰紧随其后的 `for`：删除装饰行，保留串行 `for`。如果参数包含 `gpu=True`、`collapse`、`ordered`、OpenMP pragma 字符串，则仍可删除语法，但必须报告语义 warning。
- `from python import module` / `from python import module as alias`：改写为普通 `import module` / `import module as alias`。
- `from python import pkg.sub as alias`：改写为 `import pkg.sub as alias`。如果导入的是函数签名形式，如 `from python import mod.fn(int) -> int`，Python 侧无法作为 import 语法表达，必须显式维护。
- `@python` 修饰函数：删除 `@python` 装饰器，让函数体成为普通 Python 函数；同时报告该函数在 Codon 中走 CPython interop，调试目标直接执行 Python 体。
- Codon 整数/浮点类型注解：`i8/u8/i16/u16/i32/u32/i64/u64/Int[N]/UInt[N]` 在 Python 注解中改成 `int`；`f32/f64` 改成 `float`；原始 Codon 类型保留给 guard。
- Codon 容器类型注解别名：`List[T]`、`Dict[K, V]`、`Set[T]`、`Tuple[...]` 可以降成 Python 3.12 内建泛型 `list[T]`、`dict[K, V]`、`set[T]`、`tuple[...]`，或统一保留为 `typing` 兼容形式。选择一种后全局一致。
- Python 3.12 已支持的类型参数列表：简单 `def f[T](...)`、`class C[T]: ...`、`type Alias[T] = ...` 可以原样保留；如果其中的类型注解含 Codon-only 标量类型，只改写内部注解。
- Codon 显式类型参数参数：`def f(x: list[T], T: type)` 可机械转成 Python 3.12 泛型签名并移除调用侧类型实参，推荐目标形态是 `def f[T](x: list[T])`；如果 `T` 有默认类型或函数体读取 `T` 的运行时值，则不能简单删除，必须保留一个 Python debug wrapper 或显式分叉。
- `list(capacity=N)`：改成 `list()`，容量提示只影响 Codon 性能，不影响 Python 逻辑结果；必须报告 performance-only 降级。
- `Dict[K,V]()`、`Set[T]()`、`List[T]()`：分别改成 `{}`、`set()`、`list()` 或对应构造函数；原始类型交给 guard。
- `@tuple class X:`：可改成 `@dataclass(frozen=True, slots=True)` 或 `typing.NamedTuple`。v1 推荐 `@dataclass(frozen=True, slots=True)`，因为更容易保留方法；需自动补 `from dataclasses import dataclass`。
- Codon 类字段 preamble，如类体内的 `x: int`、`y: float`：Python 3.12 可解析，可保留；若转换为 dataclass，需要作为字段保留。
- Codon 普通 `match` 中与 Python 3.12 相同的部分：字面量、`_`、`|`、tuple/list 基础解构、mapping/class pattern、guard `if` 可原样保留。
- Codon `match` 范围模式 `case 2 ... 10`：可机械转成 Python 3.12 match guard。必须先把 subject 表达式提升到临时变量，保证只求值一次，再生成 `case _ if 2 <= __codonx_match <= 10:`。
- Codon 列表省略模式，如 `[1, 2, ..., 5]`：可做 match-block 级机械转译，生成长度/前缀/后缀 guard；实现前必须用 AST 或明确的模式解析器，不允许正则猜测。

必须显式维护的语法：

- `@gpu.kernel`、`import gpu` 和 GPU intrinsic，如 `gpu.thread.x`：不能自动变成等价 Python。要求 `#%ifdebug` 分叉；未分叉时 Python 目标只允许生成 warning/注释，不应假装等价。
- `@par(gpu=True)` 的 GPU 执行语义：语法可删除，但 GPU 版本的正确性必须由 release/Codon 测试覆盖。
- `from python import mod.fn(type, ...) -> type`：Codon 的 typed Python interop 声明不是 Python import 语法，必须手写 debug 分支或降成普通 Python wrapper。
- Codon 的复杂 `match` 扩展如果不能由 match-block 解析器安全覆盖，必须显式分叉；但不能因为 Python 3.12 “不支持 match” 这种错误理由拒绝它。
- 函数/方法重载：Python 后定义会覆盖先定义，不能自动保留 Codon overload resolution。必须显式维护，或后续生成 singledispatch/manual dispatcher。
- 泛型运行时分派和显式类型实参：`class C[T]` / `def f[T]` 语法本身 Python 3.12 可解析，但 Codon 的单态化、`A[str](...)`、`T: type` 参数、默认类型实参和函数体读取类型参数的运行时行为与 Python 不同。能删除类型实参且不影响函数体的情况可机械转译；否则必须显式维护。
- `@extend class X:`：Codon 是编译期扩展类型；Python monkey patch 语义不同，且内建类型多数不能安全 patch。必须显式维护。
- OpenMP constructs：`import openmp`、`@omp.critical/master/single/ordered` 等不能等价转成普通 Python，必须显式维护或标注。
- `threading.ThreadLocal[T]` 类型语义：Python 可用 `threading.local()` 近似，但自动转译容易误改作用域；v1 必须显式维护。
- Codon-only 低层能力：指针、`cobj`、LLVM/C interop、GPU target flags、PTX 输出、SIMD/硬件 intrinsic，必须显式维护。

## 语义层面：可用 Python assert/guard 高效模拟

目标：覆盖 50% 以上常见 Codon/Python 语义差异，让 Python 调试目标尽早失败，而不是静默跑出与 Codon release 不一致的结果。

优先自动 guard：

- 整数类型和值域：`int/i64` 检查 64-bit signed；`u64` 检查非负 64-bit；`i32/u32/.../i8/u8` 按位宽检查。必须使用 `type(x) is int`，避免 Python `bool` 被当作 `int`。
- `Int[N]` / `UInt[N]`：解析常量位宽后做范围检查；位宽过大时仍可检查 Python int 范围，但要注意性能。
- 浮点类型：`float/f64` 检查 `type(x) is float`；`f32` 可检查类型但不能高效保证舍入完全一致，需 warning。
- `bool`：检查 `type(x) is bool`。
- `str`：检查 `type(x) is str`，默认检查 ASCII；如果后续增加开关，可允许关闭 ASCII guard。
- 容器外形：`list/set/dict/tuple` 的浅层检查默认开启，确认 Python 对象种类和 tuple 长度。
- 容器元素：`--assert full` 时递归检查 `list[T]`、`set[T]`、`dict[K,V]`、`tuple[T1,T2]` 元素类型和值域。
- 函数参数、显式注解局部变量、返回值：根据源码原始 Codon 类型插入 guard。
- 同质集合约束：对 Codon `List[T]`、`Set[T]`、`Dict[K,V]`，full 模式检查元素类型一致，避免 Python 混合类型集合在 Codon 编译/运行时不成立。
- tuple 固定长度：按注解检查 tuple 长度；对异质 tuple 的动态索引风险生成 warning。
- `list(capacity=N)`：只做语义注释，不做运行时检查；容量不影响结果。
- `@par` 降级：插入 warning，说明 Python 是串行 fallback，不能检测数据竞争、调度顺序、reduction 推断错误。
- `@python` 降级：插入 warning，说明 Python debug 直接执行函数，Codon release 通过 CPython interop/转换路径执行。

必须显式关注的语义：

- 并行数据竞争：Python 串行执行无法暴露 Codon `@par` 下的共享 list/dict 写入、锁使用错误、reduction 推断差异。
- OpenMP 调度和顺序：`schedule/chunk_size/num_threads/ordered/collapse` 影响性能和可观察顺序，Python 不能用 assert 高效模拟。
- GPU 语义：线程索引、block/grid、GPU 内存、CUDA/libdevice 数学、`@par(gpu=True)` 限制、shared variable 禁止等必须靠 Codon/GPU 测试。
- 数值运算细节：Codon 部分 numerics 使用 C 语义；除零、`math` 函数错误行为、溢出、浮点舍入和 `f32` 精度不能完全用普通 Python assert 模拟。必要时 release 使用 `-numerics=py` 对齐 Python，但这也不改变 int 位宽。
- dict/set 顺序：Codon dict/set 无序，Python dict 保序。Python debug 中不得依赖 dict 迭代顺序；可在检测到对 dict 直接迭代时 warning，但不能普遍证明。
- Python 动态特性：monkey patch、运行时添加属性、混合类型 collection、动态变更类结构等在 Codon 中受限；Python 可运行不代表 Codon 可编译。
- 方法重载和泛型单态化：Codon 按静态类型生成/选择实现，Python 运行时分派不同。除非后续实现 dispatcher，否则必须显式维护。
- 类值/引用差异：普通 class 是引用语义，`@tuple` 是值语义且不可变；Python dataclass/NamedTuple 只能近似，需要测试覆盖复制、赋值和 mutation 场景。
- Python interop 转换：`__to_py__`、`__from_py__`、`pyobj`、`CODON_PYTHON` 依赖的行为涉及 CPython C API；Python debug 直接调用不等价于 Codon interop 转换。
- 标准库差异：Codon 原生 stdlib 覆盖面和实现细节与 CPython 不完全相同，如 pickle 格式、datetime 时区、regex engine 等；只能按模块加专项测试/注释。

## 后续实现要求

- 生成 Python 文件时，所有自动语法降级都要尽量在原位置附近插入短注释，例如 `# codonx: @par lowered to serial Python loop`。
- 所有不能模拟的语义都要进入 JSON report；严重项应同时写入 Python 目标注释，方便 IDE 调试时看见。
- 对必须显式维护的语法，优先报错或要求 `#%ifdebug`，不要生成“看起来能跑”的错误 Python。
- 对可 guard 的语义，默认 `--assert shallow`；昂贵递归检查只在 `--assert full` 开启。
- 每新增一种降级规则，必须同时补：Codon 源样例、Python 输出样例、Codon 输出样例、report 断言、至少一个失败 guard 测试。
