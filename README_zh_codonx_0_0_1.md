# codonx 0.0.1

**Codon-first 双目标前端预处理器：同一份 `.codonx` 源文件，生成 Python 调试版和 Codon 发布版。**

> 当前 CLI 行为已更新：`codonx --dbg input.codonx` 生成默认带 assert 的 Python 调试文件；`codonx run/build ... input.codonx` 会先生成 `input_pre.codon`，再按 Codon 官方 `codon run/build` 的参数形态调用系统 Codon 编译器，结束后默认删除预处理文件。完整命令说明见 [README.md](README.md)。

codonx 的目标不是把任意 Python 自动翻译成 Codon，也不是把任意 Codon 自动翻译成 Python。它的目标更明确：

> 让开发者从一开始就写 **Codon-first** 源码，同时保留一个可用的 **Python Debug Target**，用于 IDE、断点、pytest、pdb、快速验证和语义对拍；最终再生成 **Codon Release Target**，交给 Codon 编译器运行或构建二进制。

换句话说，codonx 是一把“屠龙刀”级别的工具：对于不了解 Codon/Python 语义差异的人，它可能反噬；对于明确知道自己在做什么的人，它可以显著降低 Codon-first 项目的开发、调试和维护成本。

当前版本：**0.0.1 MVP**  
当前定位：**Linux-first / 单文件预处理 / 条件裁剪 / Python 语义护栏雏形 / Codon 编译路径透传**

---

## 1. 为什么需要 codonx？

Codon 是一个高性能 Python-like 编译器，能够把接近 Python 的代码编译成本地机器码，并支持多线程、GPU、Python 互操作、C/C++ 互操作等能力。但它不是 CPython 的 drop-in replacement，也不是 Python 的严格超集。

在真实工程中，Codon 当前会遇到几个痛点：

1. **Python 原型不能无痛迁移到 Codon**  
   Codon 与 CPython 存在语义差异，例如整数范围、字符串、dict 顺序、tuple 长度、静态类型、部分动态特性等。

2. **调试体验和 IDE 生态还不如原生 Python**  
   Codon 性能强，但复杂算法调试时，Python 的断点、变量观察、pytest、pdb、PyCharm/VS Code 体验仍然更成熟。

3. **维护两套代码会漂移**  
   如果长期维护 `foo.py` 和 `foo.codon` 两份文件，Python 原型和 Codon 优化版迟早会产生语义差异。

4. **Codon-only 语法会破坏 Python 解析**  
   例如 `@par` 装饰循环、GPU kernel、`from python import` 等，不是普通 CPython 可以直接解析或等价执行的东西。

codonx 的思路是：

```text
一份 .codonx 源文件
        ↓
  codonx 预处理
        ↓
Python Debug Target        Codon Release Target
用于调试、验证、pytest       用于 codon run/build、二进制发布、性能测试
```

---

## 2. 核心思想

### 2.1 Codon-first，而不是 Python-first

codonx 假设源文件从一开始就是 Codon-first 的。

这意味着：

- 你可以在源文件中使用 Codon 的类型信息；
- 你可以保留 Codon 的并行/GPU/互操作意图；
- 你可以在 release 分支中写高性能 Codon 代码；
- Python 只是 debug projection，不是主源语言。

因此，codonx 不追求“任意 Python 自动升级到 Codon”，而是追求：

> 以 Codon 为主源语言，向下生成一个尽量可调试、可验证、带语义护栏的 Python 版本。

---

### 2.2 用 `#%ifdebug` 管理差异点

codonx 使用 C 风格的预处理指令，而不是伪装成运行时 `if DEBUG:`。

原因很简单：Python 会先解析整个文件，再执行代码。如果原文件里含有 Python 不认识的 Codon-only 语法，即使它在永远不会执行的分支里，Python 也会直接语法错误。

所以 codonx 使用明确的预处理期指令：

```python
#%ifdebug
# Python Debug Target 使用的代码
#%else
# Codon Release Target 使用的代码
#%endif
```

示例：

```python
def square_all(xs: list[i32]) -> list[i32]:
    out: list[i32] = [0 for _ in range(len(xs))]

    #%ifdebug
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%endif

    return out
```

生成 Python Debug Target 时：

```python
def square_all(xs: list[int]) -> list[int]:
    out: list[int] = [0 for _ in range(len(xs))]

    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]

    return out
```

生成 Codon Release Target 时：

```python
def square_all(xs: list[i32]) -> list[i32]:
    out: list[i32] = [0 for _ in range(len(xs))]

    @par(schedule="dynamic", chunk_size=64)
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]

    return out
```

---

### 2.3 Python 模式只生成 `.py` 文件，不替代调试器

codonx 的 Python 模式只负责生成纯 Python 文件。

它不试图代替开发者运行调试器，也不试图包装 pytest、pdb、PyCharm 或 VS Code。原因是 Python 调试生态已经很成熟，codonx 不应该重复造轮子。

推荐流程：

```bash
codonx py src/main.codonx -o build/py/main.py
python3 build/py/main.py
pytest build/py
```

或者直接把 `build/py` 目录配置进你的 IDE，然后使用原生 Python 调试器。

---

### 2.4 Codon 模式尽量贴近 Codon 原生命令

Codon 侧的命令行规则不标新立异。

codonx 的 `run` 和 `build` 应尽量保持与 Codon 原生命令一致，降低学习成本：

```bash
codonx run src/main.codonx
codonx run -release src/main.codonx

codonx build -release -o dist/app src/main.codonx
codonx build -o dist/app src/main.codonx
```

内部流程是：

```text
.codonx
  ↓ codonx 预处理为临时 .codon
codon run / codon build
```

也就是说，Codon target 的运行/构建行为应尽量贴近：

```bash
codon run ...
codon build ...
```

codonx 只是在前面多做一步预处理。

---

## 3. 当前 0.0.1 支持情况

### 3.1 已支持：单文件输入输出

当前 MVP 以单文件为主：

```bash
codonx py examples/hello.codonx -o build/hello.py
codonx codon examples/hello.codonx -o build/hello.codon
```

建议 0.0.1 至少支持以下输出：

```text
target=py     生成 Python Debug Target
target=codon  生成 Codon Release Target
```

---

### 3.2 已支持：条件块裁剪

支持：

```text
#%ifdebug
#%else
#%endif
```

目标语义：

```text
Python Debug Target:
    保留 #%ifdebug 块
    删除 #%else 块

Codon Release Target:
    删除 #%ifdebug 块
    保留 #%else 块
```

支持嵌套条件块，但 0.0.1 不建议滥用嵌套。MVP 的核心设计原则是：差异点要少、要显式、要可审查。

---

### 3.3 已支持：缩进恢复

因为 Python 和 Codon 都是缩进敏感语言，codonx 不能只做裸正则替换。

0.0.1 的设计是：

```text
读取源文件
  ↓
按行扫描
  ↓
识别 #%ifdebug / #%else / #%endif
  ↓
按 target 裁剪
  ↓
恢复输出文本
```

后续版本会继续增强缩进块处理和 source map。

---

### 3.4 已支持：`from python import` 降级

Codon 中可以写：

```python
from python import torch
from python import pandas as pd
```

Python Debug Target 中应降级为：

```python
import torch
import pandas as pd
```

这使得 Codon 中的 Python 生态互操作，在 Python Debug Target 中仍然可以正常使用普通 Python import 语义。

---

### 3.5 已支持：`@par` 串行 fallback 和风险报告

Codon Release Target 可以保留：

```python
@par(schedule="dynamic", chunk_size=64)
for i in range(n):
    work(i)
```

Python Debug Target 中不模拟真实并行，而是降级为串行循环：

```python
for i in range(n):
    work(i)
```

同时生成 warning report，提醒：

```text
Python Debug Target 是串行 fallback
不能检测 Codon Release Target 中的并行数据竞争
共享 list/dict 写入必须在 Codon Release 环境手动测试
```

这是故意设计，不是缺陷。Python Debug Target 的职责是抓算法语义错误，不是模拟 OpenMP 并行运行时。

---

### 3.6 已支持：GPU 构造风险报告

遇到：

```python
@par(gpu=True)
for i in range(n):
    ...

@gpu.kernel
def kernel(...):
    ...
```

0.0.1 不尝试模拟 GPU。

Python Debug Target 可以选择删除 GPU-only 装饰器或要求手动 debug 分支；同时生成 warning report，提醒：

```text
GPU 语义未在 Python Debug Target 中模拟
必须在 Codon Release 环境中进行真实 GPU correctness / stress test
```

---

### 3.7 已支持：基础类型语义护栏

Codon 和 Python 的一个重要差异是整数语义。Python 的 `int` 是任意精度；Codon 的 `int` 更接近固定宽度整数语义，普通 `int` 可按 i64 处理。

因此 Python Debug Target 中，如果源文件有显式类型声明，codonx 会插入 assert 语义护栏。

示例源文件：

```python
def add_i32(a: i32, b: i32) -> i32:
    c: i32 = a + b
    return c
```

Python Debug Target 中可以生成类似：

```python
def add_i32(a: int, b: int) -> int:
    __codonx_assert_value(a, "i32", "a", full=False)
    __codonx_assert_value(b, "i32", "b", full=False)

    c: int = a + b
    __codonx_assert_value(c, "i32", "c", full=False)

    __codonx_ret = c
    __codonx_assert_value(__codonx_ret, "i32", "<return>", full=False)
    return __codonx_ret
```

0.0.1 计划支持或已经支持的基础类型护栏：

```text
int   -> Python int + i64 范围检查
i64   -> Python int + i64 范围检查
u64   -> Python int + u64 范围检查
i32   -> Python int + i32 范围检查
u32   -> Python int + u32 范围检查
i16   -> Python int + i16 范围检查
u16   -> Python int + u16 范围检查
i8    -> Python int + i8 范围检查
u8    -> Python int + u8 范围检查
float -> Python float
f32   -> Python float，精度差异暂不完整模拟
f64   -> Python float
bool  -> Python bool
str   -> Python str + 可选 ASCII 检查
```

注意：Python 的 `bool` 是 `int` 的子类，因此整数 guard 应使用 `type(x) is int`，而不是简单 `isinstance(x, int)`。

---

### 3.8 已支持：浅层容器语义护栏

0.0.1 的容器 guard 是浅层优先，full 模式可递归检查。

支持方向：

```text
list[T]
set[T]
dict[K, V]
tuple[T1, T2, ...]
```

例如：

```python
def f(xs: list[i32]) -> i32:
    ...
```

Python Debug Target 可以检查：

```text
xs 必须是 list
full 模式下，xs 中每个元素必须是 i32 范围内的 int
```

断言模式建议：

```bash
codonx py --assert off     src/main.codonx -o build/main.py
codonx py --assert shallow src/main.codonx -o build/main.py
codonx py --assert full    src/main.codonx -o build/main.py
```

默认建议为 `shallow`，因为大容器全量检查可能非常慢。

---

## 4. 推荐命令行规则

为了降低门槛，CLI 不应该过度发明新概念。

### 4.1 Python 生成模式

Python 模式只生成 `.py` 文件：

```bash
codonx py src/main.codonx -o build/py/main.py
```

可选：

```bash
codonx py --assert off src/main.codonx -o build/py/main.py
codonx py --assert shallow src/main.codonx -o build/py/main.py
codonx py --assert full src/main.codonx -o build/py/main.py
codonx py --report build/codonx_report.json src/main.codonx -o build/py/main.py
```

不建议让工具直接代替开发者调试：

```bash
# 不建议作为核心模式
codonx debug ...
```

正确做法是：

```bash
codonx py src/main.codonx -o build/py/main.py
python3 build/py/main.py
pytest build/py
```

IDE 调试也应直接调试生成的 `.py` 文件。

---

### 4.2 Codon 生成模式

如果开发者想看生成的 Codon 文件：

```bash
codonx codon src/main.codonx -o build/codon/main.codon
```

这只是 emit，不运行、不编译。

---

### 4.3 Codon run 模式

尽量贴近 Codon：

```bash
codonx run src/main.codonx
codonx run -release src/main.codonx
```

内部等价于：

```bash
codonx codon src/main.codonx -o /tmp/codonx_xxx.codon
codon run /tmp/codonx_xxx.codon
```

或者：

```bash
codon run -release /tmp/codonx_xxx.codon
```

---

### 4.4 Codon build 模式

尽量贴近 Codon：

```bash
codonx build -release -o dist/app src/main.codonx
codonx build -o dist/app src/main.codonx
```

内部等价于：

```bash
codonx codon src/main.codonx -o /tmp/codonx_xxx.codon
codon build -release -o dist/app /tmp/codonx_xxx.codon
```

如果需要透传 Codon 参数，可以放在 `--` 后面：

```bash
codonx build -release -o dist/app src/main.codonx -- -disable-exceptions
```

---

### 4.5 Check 模式

检查预处理结构，不代替完整测试：

```bash
codonx check src/main.codonx
```

建议 check 做这些事：

```text
1. 检查 #%ifdebug / #%else / #%endif 是否配对
2. 尝试生成 Python Debug Target
3. 尝试生成 Codon Release Target
4. 可选执行 python3 -m py_compile
5. 可选执行 codon 编译前检查
6. 输出 warning 摘要
```

---

## 5. 当前项目结构建议

```text
codonx/
├── Cargo.toml
├── README.md
├── codonx.toml.example
├── src/
│   ├── main.rs              # CLI 入口
│   ├── cli.rs               # 命令行参数
│   ├── error.rs             # 统一错误类型
│   ├── source.rs            # 文件读取、行号、缩进、三引号状态
│   ├── directive.rs         # #%ifdebug / #%else / #%endif
│   ├── emit.rs              # target 类型与输出逻辑
│   ├── rewrite.rs           # Python/Codon target 重写
│   ├── type_parse.rs        # 轻量类型解析
│   ├── guard.rs             # Python assert 语义护栏
│   └── report.rs            # warning/report JSON
├── examples/
│   ├── hello.codonx
│   ├── type_guard.codonx
│   └── parallel.codonx
├── tests/
│   ├── cases/
│   ├── expected_py/
│   ├── expected_codon/
│   └── golden.rs
└── scripts/
    └── build_release.sh
```

---

## 6. 构建

Linux-first。

```bash
cargo build
```

Release 二进制：

```bash
bash scripts/build_release.sh
```

生成：

```text
dist/codonx
```

---

## 7. 0.0.1 非目标

codonx 0.0.1 明确不做以下事情：

```text
1. 不做完整 Codon parser
2. 不做完整 Python parser
3. 不做任意 Codon 到 Python 的完整转译
4. 不做任意 Python 到 Codon 的迁移
5. 不模拟 GPU kernel
6. 不自动证明并行代码无数据竞争
7. 不处理 Ptr / LLVM IR / C interop 的等价降级
8. 不替代 pytest、pdb、IDE 调试器
9. 不承诺 Windows 原生支持
10. 不承诺生成的 Python Debug Target 与 Codon Release Target 在所有语义上完全一致
```

它的正确使用方式是：

```text
普通逻辑：尽量写 Python/Codon 共同子集
差异逻辑：用 #%ifdebug 显式分叉
语义差异：用 assert guard 尽量提前暴露
并行/GPU差异：用 report 显式提醒，release 环境手动测试
```

---

## 8. 设计纪律

### 8.1 差异必须显式

不要让 codonx 猜测复杂语义。

遇到不确定的 Codon-only 构造，宁愿 warning 或 error，也不要生成“看起来能跑但语义不等价”的 Python 代码。

---

### 8.2 Python Debug Target 是调试目标，不是性能目标

Python target 可以慢，可以多 assert，可以更啰嗦。它的目标是：

```text
更容易断点
更容易看变量
更容易跑 pytest
更容易发现类型/范围/返回值问题
```

它不是用来替代 Codon 性能测试的。

---

### 8.3 Codon Release Target 是性能目标，不是调试目标

Codon target 应尽量少改动源代码，尤其不要在 release target 中插入 Python 式 guard。

release target 的职责是：

```text
交给 codon run
交给 codon build
交给 Codon 的 @par/GPU/native 编译路径
```

---

### 8.4 必须对拍

任何使用 `#%ifdebug` 和 `#%else` 分叉的函数，都建议写对拍测试：

```text
同一输入
Python Debug Target 结果
Codon Release Target 结果
二者比较
```

尤其是：

```text
路径搜索
A*
clock feasibility
batch scoring
并行循环
GPU kernel
数值边界
```

---

## 9. 演进路线

### 0.0.1：MVP

目标：

```text
单文件预处理
#%ifdebug 条件裁剪
Python/Codon 双目标生成
基础 rewrite
基础 assert guard
并行/GPU warning report
Linux release 二进制
```

---

### 0.0.2：稳定 CLI 和测试

目标：

```text
统一 CLI 为：
  codonx py
  codonx codon
  codonx run
  codonx build
  codonx check

补 golden file 测试
补错误信息
补嵌套条件块测试
补 Python py_compile 检查
```

---

### 0.0.3：增强语义护栏

目标：

```text
更强的类型解析
更完整的 list/dict/tuple 嵌套检查
函数返回值 guard
局部变量 guard
--assert off/shallow/full 完整实现
ASCII str 检查开关
```

---

### 0.0.4：项目模式

目标：

```text
codonx.toml
多文件批量生成
build/py tree
build/codon tree
report 汇总
增量构建雏形
```

示例：

```toml
[project]
name = "my-codon-project"
src = "src"
build = "build"

[python]
assert = "shallow"

[codon]
release = true
```

---

### 0.1.0：可开源使用版本

目标：

```text
文档稳定
examples 可运行
CI 覆盖 Linux
release 二进制
错误报告可读
report.json/report.md
```

---

### 0.2.0：Codon 自举实验

目标：

```text
用 Codon/CodonX 重写核心预处理器
Rust 版本作为 bootstrap
Codon 版本与 Rust 版本输出对拍
最终实现 self-host
```

自举验证标准：

```text
旧 codonx 处理新版 codonx 源码
新版 codonx 处理同一份源码
输出一致
测试通过
```

---

## 10. 适合人群

适合：

```text
1. 明确想用 Codon 写性能核心的人
2. 愿意从一开始写 Codon-first 源文件的人
3. 需要 Python IDE/debugger 体验的人
4. 能理解 Python/Codon 语义差异的人
5. 能接受显式 debug/release 分支的人
6. 会为 release target 写对拍测试的人
```

不适合：

```text
1. 想把普通 Python 项目一键变成 Codon 的人
2. 不理解固定宽度整数和 Python int 差异的人
3. 希望自动模拟 GPU/并行行为的人
4. 希望任意第三方库自动兼容的人
5. 不想写测试的人
```

---

## 11. 一句话总结

codonx 0.0.1 是一个 Codon-first 的轻量前端预处理器。

它做的事情很简单：

```text
.codonx
  ↓
Python Debug Target：可调试、可断点、可 pytest、带语义 assert
Codon Release Target：可交给 codon run/build、保留并行/GPU/优化语义
```

它不试图消灭 Python 和 Codon 的差异，而是把差异显式化、局部化、可测试化。

这就是 codonx 的核心价值。
