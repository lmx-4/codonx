# codonx 中文说明

codonx 是一个 **Codon-first 双目标前端预处理器**。

它的目标是让一份 `.codonx` 源文件同时生成：

- **Python Debug Target**：用于 IDE、断点、pytest、pdb、快速验证；
- **Codon Release Target**：用于 `codon run`、`codon build`、二进制发布和性能测试。

codonx 不是普通转译器，也不是 Python 到 Codon 的迁移器。它是一把面向 Codon power users 的“屠龙刀”：非常锋利，但要求使用者理解 Python/Codon 的语义差异。

## 核心原则

### 1. Codon-first

源文件从一开始就应该按 Codon-first 的方式写。

也就是说，`.codonx` 源码可以携带 Codon 的类型信息、并行信息、GPU 信息和 Python interop 信息。Python 只是调试目标，不是源语言本体。

### 2. 差异显式化

Python 和 Codon 不一致的地方，不应该藏起来。

使用：

```python
#%ifpy
# Python 调试代码
#%else
# Codon 发布代码
#%endif
```

或者：

```python
#%ifcodon
# Codon 发布代码
#%else
# Python 调试代码
#%endif
```

旧的 `#%ifdebug` 仍然兼容，但新代码建议使用 `#%ifpy`。

### 3. Python 模式只生成 `.py`

Python 模式不替代开发者调试。

推荐流程：

```bash
codonx --dbg test.codonx -o build/test.py
python3 build/test.py
pytest build
```

后续 CLI 可以逐步调整为更清晰的：

```bash
codonx py test.codonx -o build/test.py
```

### 4. Codon 模式贴近原生 Codon

Codon 运行/构建应尽量贴近原生命令：

```bash
codonx run -release test.codonx
codonx build -release -o dist/app test.codonx
```

codonx 只是先把 `.codonx` 预处理成 `.codon`，然后调用 Codon 编译器。

## 当前能力边界

当前 0.0.x 支持：

- `#%ifpy`
- `#%ifcodon`
- `#%else`
- `#%endif`
- deprecated `#%ifdebug`
- `#%define CODON_PYTHON`
- `#%define CODON_DEBUG`
- 部分 Python debug 降级
- 基础 runtime semantic guards
- `codon run/build` 薄包装

当前不支持：

- 完整 Codon parser
- 完整 Python parser
- 任意 Codon 到 Python 的完整转译
- GPU 模拟
- OpenMP race 检测
- Ptr/C/LLVM interop 自动降级
- 证明 Python/Codon 语义等价

## 类型语义护栏

Codon 的常规数值路径接近 Python 语法：优先使用 `int` 和 `float`。
其中 `int` 是 64 位有符号整数，`float` 是 64 位浮点数。

`i32`、`u64`、`f32` 等是低层固定宽度类型，适合你明确需要位宽语义时使用。

因此 Python Debug Target 可以插入断言，例如：

```python
_codonx_assert_value(x, "i32", "x", full=False)
```

用于提前发现：

- `i32` 越界；
- `u64` 出现负数；
- `bool` 被误当成 `int`；
- `str` 出现非 ASCII；
- 容器元素类型明显不匹配。

这些 guard 不是等价性证明，只是早期错误探测器。

## 并行和 GPU

Python Debug Target 不模拟 Codon 的真实并行/GPU 行为。

遇到：

```python
@par
for i in range(n):
    work(i)
```

Python target 会保留串行循环，并写入 warning/report。

真实并行 correctness 必须在 Codon Release Target 中测试。

## 适合人群

适合：

- 想从一开始写 Codon-first 项目的人；
- 需要 Python 调试体验的人；
- 能理解 Python/Codon 语义差异的人；
- 愿意为 release target 写对拍测试的人。

不适合：

- 想一键把普通 Python 变成 Codon 的人；
- 想自动模拟 GPU/并行行为的人；
- 不想处理语义边界的人。

## 一句话总结

codonx 不消灭 Python/Codon 差异。

它把差异显式化、局部化、可测试化。
