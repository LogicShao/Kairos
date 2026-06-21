# Input Patterns — 数字输入组件选型指南

> 为不同场景选择最合适的数字输入控件。

---

## 决策速查

| 场景 | 推荐组件 | 本项目位置 |
|------|---------|-----------|
| 番茄钟时长 (5min步进) | **Stepper 步进器** | `src/components/ui/stepper.tsx` |
| 周次选择 (1-20) | Dropdown / Bottom Sheet | — |
| 时间点 (14:30) | 系统 TimePicker | — |
| 固定选项 (15m/25m/45m) | 预设按钮网格 | — |
| PIN/自定义数字 | 自定义数字键盘 | — |

---

## 一、Stepper 步进器

**形式**：`[-] 当前值 [+]`，支持长按连加/连减。

**适用**：数字范围小到中等（1-120），需要微调或步进。

**用法**：
```tsx
import { Stepper } from "@/components/ui/stepper"

<Stepper
  value={minutes}
  onChange={setMinutes}
  min={1}
  max={120}
  step={5}
/>
```

**特性**：
- 长按 ± 按钮持续连加/连减（120ms 间隔）
- 达到边界自动禁用按钮（opacity-30）
- 移动端触摸目标 ≥ 44dp（min-h-11 min-w-11）
- 中间显示当前值，等宽字体（tabular-nums）

---

## 二、滑块 (Slider)

**适用**：精确度要求不高，侧重调整幅度/感受（音量、优先级 1-10）。

**不足**：不适合选精准个位数（如精确到 17）。

> 本项目暂未内置 Slider 组件。需要时基于 Radix Slider 或 Tailwind 手写。

---

## 三、下拉菜单 / 底部弹窗列表

**适用**：选项明确且有限（第1-20周、学期列表、预设提醒时间）。

**本项目已有**：
- 课程表周次选择器（popover）
- 学期选择（overflow menu submenu）

---

## 四、滚轮选择器 (Wheel Picker)

**适用**：日期（年月日）或时间（时分）选择，类似 iOS 风格。

**不足**：占用屏幕空间大，大范围滑动累。

> 移动端优先使用系统 TimePicker。桌面端可用原生 `<input type="time">`。

---

## 五、时钟表盘 (Clock Face Picker)

**适用**：专门选择"时:分"，Android Material Design 经典。

**交互**：弹出表盘 → 点小时 → 点分钟 → 确认。

> Android 系统自带，Tauri 可调用 Android 原生 TimePicker。

---

## 六、自定义数字键盘

**适用**：需要输入数字但不想弹出系统键盘（PIN、计算器、金额）。

**优点**：样式可控不遮挡，物理杜绝非法字符。

**构建**：3×4 Grid（0-9 + 退格），主题色与 App 统一。

> 本项目暂未内置。需要时在 `src/components/ui/numpad.tsx` 创建。

---

## 反模式

| ❌ 不要 | ✅ 应该 |
|--------|--------|
| 对番茄钟时长用 `type="number"` 输入框 | 用 Stepper，5 分钟步进 |
| 对周次用自由文本输入 | 用下拉/滚轮，20 周封顶 |
| 系统键盘顶起页面布局 | 用自定义键盘或 Bottom Sheet |
| Slider 选精确数字 | 用 Stepper 或输入框 |
