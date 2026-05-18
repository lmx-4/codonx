# codonx 中文说明

`codonx` 是一个 **Linux-only、Codon-first、双目标前端预处理器**。

它解决的问题很具体：很多项目一开始用 Python 写，因为调试方便；真正需要性能时再迁到 Codon。但这样很快会变成两份文件、两套思路、两条逐渐漂移的逻辑。`codonx` 的做法不是假装 Python 和 Codon 完全一样，而是把差异显式写在同一份源码里。

```text
一份 Codon-first 源码
        |
        +-- 生成 Python 3.12+ Debug Target
        |       用于 IDE、pdb、pytest、断点和运行期 guard
        |
        +-- 生成/运行 Codon Release Target
                交给真实 codon 编译器执行
```

当前版本：**0.2.2 Ruff frontend + conservative Codon candidate MVP / experimental**。

当前主线是 **0.2.x Ruff frontend + CodonX IR**：用 Ruff 解析 Python
3.12，把 `#%` 注释宏绑定到 AST 节点，在 CodonX IR 中把代码分成
`codon_native`、`guarded`、`fallback`、`unsupported`，并开始生成保守的
Codon candidate。详细规则见
[roadmap-0.2.x.md](roadmap-0.2.x.md)。

## 硬性环境要求

0.1.4 不追求“到处能跑”，它只支持一条明确路线。

- **系统：** 仅支持 Linux。
- **Python：** 必须有 Python 3.12 或更新版本，用于运行 debug 输出。
- **Codon：** 必须有官方 `codon` 编译器，用于 `codonx run`、`codonx build` 和 release 验证。
- **Rust：** 只有从源码构建 `codonx` 时才需要。

没有 Codon 编译器时，`codonx` 的核心工作流是不完整的。它不是 Codon 的替代品，而是围绕 Codon 编译器的一层轻量预处理。

## 源码形态

新项目建议使用普通 `.codon` 后缀。仓库里的示例仍保留 `.codonx`，主要是为了让“这是 codonx 方言示例”更明显；CLI 本身不依赖后缀。

一份 codonx 兼容源码可以包含：

- Codon/Python 风格类型：`int`、`float`、`list[int]`、`dict[str, int]`。
- 明确低层位宽类型：`i32`、`u64`、`f32`、`Int[32]`。
- Codon-only 构造：`@par`、`@llvm`、`@extend`、Python interop。
- 目标分支指令：`#%ifpy`、`#%ifcodon`、`#%else`、`#%endif`。
- Codon 子进程环境指令：`#%define CODON_PYTHON`、`#%define CODON_DEBUG`。

0.2.x 会开始为更长期的 Python/Codon 双向投影打基础，但第一步不是承诺
“任意 Python 自动转 Codon”，而是先建立稳定 IR、宏绑定、可转译性分析和
明确 fallback 诊断。

## 最小示例

```python
def add(a: int, b: int) -> int:
    c: int = a + b
    return c

print(add(1, 2))
```

生成 Python Debug Target：

```bash
codonx --dbg hello.codon -o hello.py
python3.12 hello.py
```

走 Codon Release Target：

```bash
codonx run -release hello.codon
codonx build -release -o dist/hello hello.codon
```

0.2.x 的第一批实验命令已经独立接入：

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
codonx py-codon app.py -o app.codon
codonx py-run app.py
python3.12 app_assert_ir.py
```

`ir` 用 Ruff 解析 Python 3.12 源码并输出当前 CodonX AST 视图的 JSON
调试 dump；`assert-ir` 生成可执行的 Python 语义 IR：保留程序形状，并围绕
已支持的注解和返回值插入面向 Codon 语义的运行期 guard。`py-codon`
生成保守 Codon candidate：被标记为 Codon 原生的 import 保持原样，默认
Python import 走 Codon 的 `from python import ...` 互操作回退，控制宏会被
剥离，其余 Python/Codon 共同子集尽量保持源码形态。

`py-codon` 的第一目标是“能编译、能跑”，不是完整 Python-to-Codon 转译器。
如果生成文件使用 Python fallback import，需要在 Codon 编译/运行进程中注入
Python 运行时路径。源码中的
`#%define CODON_PYTHON "/path/to/libpython3.12.so"` 会被写入生成文件头部，
作为应注入的 `CODON_PYTHON` 值。`py-run` 和 `py-build` 会在调用真实
Codon 编译器前自动完成这一步环境注入。

0.2.x 对 Python import 采用兼容优先：默认把导入视为 CPython fallback
候选。如果某个 import 必须使用 Codon 原生库，在它前面加 `#%codon`：

```python
#%codon
import math

import numpy as np  # 规划为 CPython fallback
```

如果 `codon` 不在 `PATH`：

```bash
codonx --codon-bin /opt/codon/bin/codon run -release hello.codon
```

也可以使用环境变量：

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## 核心机制：差异显式化

Python 和 Codon 的差异不应该藏在魔法转换里。需要不同实现时，直接写目标分支：

```python
def square_all(xs: list[int]) -> list[int]:
    out: list[int] = [0 for _ in range(len(xs))]

    #%ifpy
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%endif

    return out
```

Python debug 输出保留串行循环；Codon release 输出保留 `@par` 并行循环。

`#%ifdebug` 仍作为旧别名兼容，但新代码应该使用 `#%ifpy`。

## 0.1.4 的类型语义

0.1.4 延续 Codon 的常规数值风格：主线代码优先写 `int` 和 `float`。

```python
def mean(xs: list[int]) -> float:
    total: int = 0
    for x in xs:
        total += x
    return float(total) / float(len(xs))
```

`i32`、`u64`、`f32` 这类类型不是默认推断目标，而是明确的低层位宽意图：

```python
def clamp_i32(x: i32) -> i32:
    return i32(x)
```

Python debug guard 会尽量检查这些范围，但 release 语义仍由 Codon 编译器决定。如果 Codon 拒绝 `int` 和 `i32` 混合运算，就应该改源码，而不是让 `codonx` 猜。

## Python Debug Target

0.1.4 使用顶层 `--dbg` 生成 Python 文件，没有 `codonx py` 子命令。

```bash
codonx --dbg input.codon -o input_dbg.py
codonx --dbg input.codon --assert full -o input_dbg.py --report codonx-report.json
python3.12 input_dbg.py
```

断言模式：

```text
off
shallow
full
```

默认是 `shallow`。

guard 能覆盖的主要范围：

- 整数和固定宽度整数范围。
- `float`、`f32`、`f64`、`float32`。
- `bool`、`complex`、ASCII `str`。
- `Optional`、`Union`、软化的 `Literal`。
- `list`、`set`、`dict`、`tuple` 的外层形状，`--assert full` 下检查部分元素。

这些 guard 是错误探测器，不是等价性证明。

## Codon Release Target

```bash
codonx codon input.codon -o input_pre.codon
codonx run -release input.codon
codonx build -release -o dist/app input.codon
```

`run` 和 `build` 是薄包装：先做指令选择和预处理，再调用真实 `codon run` 或 `codon build`。

保留中间 Codon 文件：

```bash
codonx --keep-pre run -release input.codon
```

## 当前实现边界

0.1.4 已经不是纯正则替换。语义改写收口到局部 AST/span 和 token-aware 机制；正则不再作为机械语义改写的最终依据。

当前适合：

- 结构清晰的 Codon-first 单文件。
- 显式写出 Python/Codon 差异的代码。
- 用 Python debug target 进行早期错误检查。
- 用真实 Codon 编译器验证 release 路径。

当前不适合：

- 任意 Python 自动转 Codon。
- 任意 Codon 自动转 Python。
- 完整 Codon parser。
- 完整 Python parser。
- GPU、并行 race、LLVM、C pointer、JIT 语义模拟。
- 依赖 `codonx` 证明 Python/Codon 行为完全等价。

## 一句话总结

`codonx` 不消灭 Python/Codon 差异。

它把差异显式化、局部化、可测试化，并把 release 路径交还给真正的 Codon 编译器。
