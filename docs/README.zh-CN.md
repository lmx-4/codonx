# codonx 中文说明

`codonx` 解决的是一个很现实的问题：Python 好调试，Codon 适合 release，
但项目一旦同时维护 `.py` 和 `.codon` 两份文件，逻辑很快就会漂移。

`codonx` 的目标不是假装 Python 和 Codon 完全一样，而是把两者的差异变成
可见、局部、可测试的工程边界。

```text
Codon-first 源码
    -> Python 3.12+ debug 文件，带运行期 guard
    -> Codon release 文件，交给真实 codon 编译器

Python 3.12 源码，0.2.x 实验路径
    -> Ruff parser frontend
    -> CodonX debug/semantic IR
    -> 保守 Codon candidate 或 CPython fallback import
```

当前版本：**0.2.3 experimental**。

当前可靠主线仍是 0.1.x 建立的 Codon-first 工作流。0.2.x 新增 Ruff-backed
Python 前端和第一版保守 `py-codon` bridge，但它还不是通用
Python-to-Codon 转译器。

## 它想讲的故事

你写一份本来就准备跑在 Codon 下的源码：

```python
def square_sum(xs: list[int]) -> int:
    total: int = 0
    for x in xs:
        total += x * x
    return total

print(square_sum([1, 2, 3]))
```

开发时生成 Python debug 文件：

```bash
codonx --dbg app.codon -o app_dbg.py
python3.12 app_dbg.py
```

发布时走 Codon：

```bash
codonx run -release app.codon
```

如果 Python 和 Codon 必须用不同实现，就直接写出来：

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

Python debug 文件保留串行循环；Codon release 文件保留 `@par` 循环。
这不是隐藏魔法，而是显式目标分支。

## 硬性要求

- 仅支持 Linux。
- 需要 Python 3.12 或更新版本。
- `codonx run`、`codonx build`、`py-run`、`py-build` 需要官方 `codon`
  编译器。
- 从源码构建时才需要 Rust。

`codonx` 不是 Codon 替代品。它围绕真实 Codon 编译器做预处理、检查和候选
文件生成。

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

检查外部工具：

```bash
python3.12 --version
codon --version
```

如果 `codon` 不在 `PATH`：

```bash
codonx --codon-bin /opt/codon/bin/codon run -release app.codon
```

或：

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## 主工作流：Codon First

生成 Python debug 文件：

```bash
codonx --dbg input.codon -o input_dbg.py
python3.12 input_dbg.py
```

生成更强 guard 的 debug 文件：

```bash
codonx --dbg input.codon --assert full -o input_dbg.py --report codonx-report.json
```

只生成预处理后的 Codon 文件：

```bash
codonx codon input.codon -o input_pre.codon
```

通过真实 Codon 编译器运行或构建：

```bash
codonx run -release input.codon
codonx build -release -o dist/app input.codon
```

检查指令结构和生成的 Python 语法：

```bash
codonx check input.codon
codonx check --assert full input.codon
```

`check` 不是 Codon 类型检查器。release 语义仍以真实 `codon` 为准。

## 实验工作流：Python Frontend

0.2.x 开始从 Python 方向进入：源码先经过 Ruff 的 Python 3.12 parser，再变成
CodonX 前端数据。

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
codonx py-codon app.py -o app.codon
codonx py-run app.py
codonx py-build app.py
```

0.2.3 的真实行为很保守：

- `ir` 输出 Ruff-backed CodonX 视图的 JSON debug dump。
- `assert-ir` 输出合法 Python 代码，并围绕已支持注解、赋值和返回值插入
  Codon-facing runtime guard。
- `py-codon` 输出 compile-first Codon candidate。
- `py-run` 和 `py-build` 生成 candidate，把支持的 `#%define` 注入 Codon
  子进程，调用 `codon run` 或 `codon build`，未设置 `--keep-pre` 时删除临时
  candidate。

import 规则是兼容优先。普通 Python import 会变成 Codon Python interop：

```python
import json as pyjson
```

会变成：

```python
from python import json as pyjson
```

如果 import 必须保持 Codon 原生语义，需要显式标记：

```python
#%codon
import math
```

`#%define CODON_PYTHON /path/to/libpython3.12.so` 会出现在生成 candidate 的
头部，并由 `py-run` / `py-build` 自动注入。

这条路径目前保留剩余 Python/Codon 共同子集，不承诺把任意 Python 语义降到
原生 Codon。

## 源码指令

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

`#%ifdebug` 仍等价于 `#%ifpy`，但新代码应该使用 `#%ifpy`。

Codon 子进程 hook：

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

0.2.x Python import 意图：

```text
#%codon
```

`#%codon` 必须紧贴在它描述的 import 前面。

## Guard 语义

Python debug target 可以为已支持注解插入运行期 guard。

当前支持的意图包括：

- `int`、`float`、`bool`、`complex`、ASCII `str`。
- `i32`、`u64`、`Int[32]`、`UInt[64]`、`byte` 等固定宽度整数意图。
- `f32`、`f64`、`float32` 等浮点别名。
- `Optional`、`Union`、软化的 `Literal`、`NoneType`。
- `list`、`set`、`dict`、`tuple` 的外层形状，`--assert full` 下做更深元素检查。

这些 guard 是错误探测器，不是等价性证明。它们不会模拟并行 race、GPU 执行、
Codon overload resolution、Python interop 转换细节或完整浮点差异。

## 当前适合什么

适合：

- Codon-first 单文件程序。
- 显式写出 Python/Codon 差异。
- 生成 Python 3.12 debug 文件做早期检查。
- 把 release 路径交给真实 Codon 编译器。
- 在能接受 fallback 的前提下实验 Python -> Codon candidate。

不适合：

- 任意 Python 自动转 Codon。
- 任意 Codon 自动转 Python。
- 完整 Codon parser。
- 全程序类型推断。
- 模拟 LLVM、C pointer、GPU、JIT 或并行 race 语义。
- 把 debug 输出当成 release 等价性证明。

项目规则很简单：如果一个改写不能局部、可解释、可测试，`codonx` 就应该保留、
警告，或要求你写显式分支。

## 更多文档

- [设计说明](design.md)
- [0.1.x roadmap](roadmap-0.1.x.md)
- [0.2.x roadmap](roadmap-0.2.x.md)
- [示例](../examples/README.md)
