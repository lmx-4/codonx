# codonx 中文说明

`codonx` 是 Python 可调试性与 Codon release 路径之间的一层受控桥接。

Python 和 Codon 足够接近，可以共享许多代码形态；但二者并不等价，不能依靠
隐式转换机制消除语义差异。实际项目中常见的问题是：保留一份 Python 文件用于
调试，再保留一份 Codon 文件用于性能路径。随着迭代推进，两份文件逐渐漂移，
同一算法最终变成两个程序。

`codonx` 的目标是把这条边界显式化、机械化、可测试化。它生成面向 Python 的
debug artifact，生成面向 Codon 的 release artifact，并把最终执行交还给真实
Codon 编译器。它只自动处理局部、可解释、可测试的转换；不确定的部分必须成为
显式分支、guard、fallback 或诊断。

```text
Codon-first 源码
    -> Python 3.12+ debug projection，可插入运行期 guard
    -> Codon release projection，由官方 codon binary 编译

Python 3.12 源码，0.2.x 实验前端
    -> Ruff parser frontend
    -> CodonX debug view / executable assert IR
    -> conservative Codon candidate，带显式 Python interop fallback
```

当前版本线：**0.2.3 experimental**。

当前可依赖的主线仍是 0.1.x 建立的 Codon-first workflow。0.2.x 新增
Ruff-backed Python frontend 和第一版 compile-first `py-codon` candidate
generator。这是架构基础，不是完整 Python-to-Codon 转译承诺。

## 基本契约

`codonx` 遵循四条约束。

- 同一份源码应当显式描述 Python/Codon 的差异。
- Python debug 输出用于尽早发现错配，不用于证明语义等价。
- Codon release 行为由官方 Codon 编译器负责。
- 自动 lowering 必须保守、可解释、可测试。

当 Python 和 Codon 需要不同实现时，应使用目标分支：

```python
def fill(out: list[int], n: int):
    #%ifpy
    for i in range(n):
        out[i] = i * i
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(n):
        out[i] = i * i
    #%endif
```

Python projection 保留串行循环，Codon projection 保留 `@par` 循环。差异在
源码中可见，也可以分别测试。

## 环境要求

- Linux。
- Python 3.12 或更新版本。
- `codonx run`、`codonx build`、`py-run`、`py-build` 需要官方 `codon`
  编译器。
- 从源码构建 `codonx` 时才需要 Rust。

`codonx` 不是独立编译器。它是围绕 Python 3.12+ 和 Codon 的预处理与投影层。

## 安装

```bash
tar -xzf codonx-v0.2.3-x86_64-linux.tar.gz
install -m 0755 codonx-v0.2.3-x86_64-linux/codonx ~/.local/bin/codonx
codonx --version
```

期望输出：

```text
codonx 0.2.3
```

检查外部工具链：

```bash
python3.12 --version
codon --version
```

如需显式指定 Codon：

```bash
codonx --codon-bin /opt/codon/bin/codon run -release app.codon
```

或：

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## Codon-First 工作流

以 Codon-oriented 源码为入口：

```python
def square_sum(xs: list[int]) -> int:
    total: int = 0
    for x in xs:
        total += x * x
    return total

print(square_sum([1, 2, 3]))
```

生成 Python debug projection：

```bash
codonx --dbg app.codon -o app_dbg.py
python3.12 app_dbg.py
```

生成带更强 guard 的 debug projection：

```bash
codonx --dbg app.codon --assert full -o app_dbg.py --report codonx-report.json
```

只生成 Codon projection：

```bash
codonx codon app.codon -o app_pre.codon
```

通过官方 Codon 编译器运行或构建：

```bash
codonx run -release app.codon
codonx build -release -o dist/app app.codon
```

检查指令结构和生成的 Python 语法：

```bash
codonx check app.codon
codonx check --assert full app.codon
```

`check` 不是 Codon 类型检查器。它只验证 `codonx` 预处理表面；release 语义
仍以真实 `codon` 为准。

## Python Frontend 工作流

0.2.x 从 Python 方向建立另一条入口：Python 源码先经过 Ruff 的 Python 3.12
parser，再进入 CodonX 前端 artifact。

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
codonx py-codon app.py -o app.codon
codonx py-run app.py
codonx py-build app.py
```

0.2.3 的行为范围刻意保持很窄。

- `ir` 输出 Ruff-backed CodonX view 的 JSON debug dump。
- `assert-ir` 输出可执行 Python 代码，并为受支持的基础 Python 注解插入
  guard：`int`、`float`、`bool`、`str`、`list`、`dict`、`tuple`、`set`。
- `py-codon` 输出 conservative Codon candidate。
- `py-run` 和 `py-build` 生成 candidate，把支持的 `#%define` 注入 Codon
  子进程，调用 `codon run` 或 `codon build`，并在未设置 `--keep-pre` 时删除
  临时 candidate。

import 策略是兼容优先。未标记的 Python import 会走 Codon Python interop：

```python
import json as pyjson
```

生成：

```python
from python import json as pyjson
```

Codon 原生 import 意图必须显式声明：

```python
#%codon
import math
```

`#%define CODON_PYTHON /path/to/libpython3.12.so` 会写入生成 candidate 的头部，
并由 `py-run` / `py-build` 自动注入。

当前 generator 会把剩余 Python/Codon 共同子集作为源码文本保留。它还不会把
任意 Python statement lowering 到原生 Codon 语义。

## 指令

目标选择：

```text
#%ifpy
#%ifcodon
#%else
#%endif
```

兼容别名：

```text
#%ifdebug
```

`#%ifdebug` 仍作为 `#%ifpy` 的别名保留；新代码应使用 `#%ifpy`。

Codon 子进程配置：

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

Python frontend import 意图：

```text
#%codon
```

`#%codon` 必须紧贴在它描述的 import 前面。

## Guard 边界

Codon-first Python debug 输出支持高层 Python-like 类型和显式低层 Codon 意图，
包括 `i32`/`u64`、`Int[N]`/`UInt[N]`、部分浮点别名、`Optional`、`Union`、
软化的 `Literal` 和常见容器形状。

Ruff-backed `assert-ir` 当前只保护上文列出的基础 Python 注解族。0.2.3 中这
个范围比 Codon-first debug guard 更窄，这是有意的。

所有 guard 都是错配探测器。它们不会模拟并行 race、GPU 执行、LLVM、C pointer
行为、Codon overload resolution 或 Python interop 转换语义。

## 当前适用范围

适合：

- Codon-first 单文件工作流。
- 面向 Codon-oriented 代码的 Python 3.12+ debug projection。
- 显式 Python/Codon 目标分支。
- 用 guard 尽早发现常见类型和形状错配。
- 能接受 CPython fallback import 的 Python frontend 实验。

不应当把 0.2.3 视为：

- 通用 Python-to-Codon 转译器。
- 通用 Codon-to-Python 转译器。
- 完整 Codon parser。
- 全程序类型推断器。
- Codon 并行、GPU、LLVM、C interop 或 JIT 行为模拟器。

## 更多文档

- [设计说明](design.md)
- [0.1.x roadmap](roadmap-0.1.x.md)
- [0.2.x roadmap](roadmap-0.2.x.md)
- [示例](../examples/README.md)
