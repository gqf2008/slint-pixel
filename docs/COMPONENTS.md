# slint-pixel 组件 API 文档

本页是 `slint-pixel` 组件库的 API 参考与示例。所有组件通过 `@slint_pixel` 导入，主题色引用全局 `PixelTheme`，支持 `light` / `dark` 两种方案。

## 快速开始

```slint
import { PixelButton, PixelChart, PixelTheme } from "@slint_pixel";

export component Demo inherits Window {
    preferred-width: 400px;
    preferred-height: 300px;
    background: PixelTheme.bg;

    VerticalLayout {
        spacing: 12px;
        PixelButton {
            text: "点击";
            clicked => { PixelTheme.scheme = PixelTheme.scheme == "dark" ? "light" : "dark"; }
        }
        PixelChart {
            data: [40.0, 80.0, 55.0, 90.0];
            labels: ["A", "B", "C", "D"];
        }
    }
}
```

## 主题

`PixelTheme` 是全局主题，设置 `scheme` 后所有组件自动切换配色：

```slint
PixelTheme.scheme = "dark"; // 或 "light"
```

也可在 Rust 侧设置：

```rust
slint::global!::<slint_pixel::PixelTheme>(ui.window())
    .set_scheme("dark".into());
```

### 全部主题 token

| token | 默认 | 说明 |
| --- | --- | --- |
| `bg` / `panel` / `hover` | `#ffffff` / `#ffffff` / `#f2f2f2` | 背景 / 面板 / 悬停面 |
| `edge` / `border-soft` | `#1a1a1a` / `#525252` | 边框与分隔线 / 输入类更细的描边 |
| `shadow` | `#000000` | 硬阴影层与深色块 |
| `text` / `dim` | `#000000` / `#525252` | 主文字 / 次要文字 |
| `accent` / `on-accent` | `#000000` / `#ffffff` | 强调、选中底色 / **accent 底之上的前景色**（选中行文字、勾选标记、开关滑块） |
| `danger` / `success` / `warning` / `info` | 红 / 绿 / 琥珀 / 蓝 | 功能色；`PixelAlert` 按 `kind` 取用 |
| `primary-face` / `primary-text` | `#ffffff` / `#000000` | `PixelButton variant="primary"` 的面与字 |
| `border-width` / `primary-border-width` / `border-thin` | `2px` / `2px` / `1px` | **两级描边**：外框（容器/卡片/按钮/primary）2px；内框（输入类、小件控件、组件内部线框）1px |
| `radius` / `radius-sm` | `0px` / `radius / 2` | 容器圆角 / 小件圆角（默认直角像素风） |
| `window-radius` | `0px` | 窗口自身圆角（默认直角窗口）；> 0 需配合 `PixelWindowBody` 与透明窗口，见下 |

`radius` 是全库生效的：卡片、面板、按钮、输入框、下拉、表格、对话框、标签等可见表面都读它，
复选框 / 开关 / 滑块 / 标签等小件读 `radius-sm`。**默认 0px，不设置就没有外观变化**。
单个组件仍可用自己的 `border-radius` 属性覆盖。

`radius-sm` 默认绑定 `radius / 2`，所以只写 `radius` 就能让小件自动跟随（预设与主题编辑器都只写
`radius`）；只有宿主显式给 `radius-sm` 赋过值，这个派生才会断开。

> 贴边子元素与圆角：Slint 的 `clip` 会跟随 `border-radius`（实测 2026-09-29：`PixelAlert`
> 左侧色条去掉 `clip: true` 后就会从圆角处露出直角）。自己往圆角表面里塞贴边子元素时，
> 要么给表面开 `clip: true`，要么把子元素从边缘内缩。

### 一键预设 `PixelPresets`

```slint
import { PixelPresets, PixelTheme } from "@slint_pixel";

init => { PixelPresets.soft(); }   // 暖米底 + 墨黑描边 + 暖黄主色 + 12px 圆角
// PixelPresets.classic();         // 经典黑白像素风（= 库默认值）
// PixelPresets.dark();            // 深色 + 琥珀主色

// 预设之后照样可以微调单个 token
PixelTheme.radius = 8px;
```

预设只写 `PixelTheme`，不动任何组件属性；要纯黑白就把 `success` / `warning` / `info` 设回
`PixelTheme.text` / `PixelTheme.edge`，或直接 `PixelPresets.classic()`。

### 圆角的可验证性

圆角是否"真的生效"由渲染级测试守住（不是只断言属性赋值）：

```sh
cargo test -p slint-pixel --test theme_radius          # 行为守卫 + 全库覆盖守卫
cargo run -p slint-pixel-demo --example render_gallery -- /tmp/shots   # 无头导出各预设截图
```

### 圆角窗口 `PixelWindowBody`

无边框窗口默认是直角矩形。要让窗口本身圆角，三步（缺一不可）：

```rust
// 1) 透明窗口（必须在创建窗口之前设置 backend）
slint::BackendSelector::new()
    .with_winit_window_attributes_hook(|attrs| slint_pixel::transparent_window(attrs))
    .select()?;
```

```slint
import { PixelWindowBody, PixelTheme } from "@slint_pixel";

export component App inherits Window {
    no-frame: true;
    background: transparent;                  // 2) 窗口底色透明
    init => { PixelTheme.window-radius = 12px; }   // 0px（默认）= 直角窗口、全出血
    PixelWindowBody {                         // 3) 内容包进圆角窗身
        VerticalLayout { /* 标题栏 + 内容 */ }
    }
}
```

`window-radius > 0`：四周留 3px 透明边距、按 `border-width` 描边、内容被裁在圆角内
（自绘标题栏会顺着圆角切）。`window-radius = 0`：`pad = 0`、无描边，与旧版全出血观感一致。
**未开透明窗口却设 `window-radius > 0`**，圆角外那圈边距会显示成黑边 —— 所以
`PixelPresets.*` 不写 `window-radius`，由宿主自行决定（demo 里是切换预设时一起设的）。

### 描边分成两级（外框 / 内框）

| token | 默认 | 用在哪 |
| --- | --- | --- |
| `border-width` | `2px` | **每个组件自己的可见外框**：容器/覆盖层（`PixelPanel` / `PixelCard` / `PixelDialog` / `PixelNavbar` / `PixelStat` / `PixelAlert` / `PixelToast` / `PixelTooltip` / `PixelBubble` / `PixelPopconfirm` / `PixelDrawer(Pro)` / `PixelScrollPanel` / 圆角窗身），以及按钮、菜单、表格、手风琴、折叠、树、列表、日历、看板、向导、编辑器、图表、地图、媒体等所有组件的可见外框 |
| `primary-border-width` | `2px` | `PixelButton variant="primary"`（0.2.4 起与普通按钮同宽，不再默认粗一档） |
| `border-thin` | `1px` | **输入类与小件自己的框 + 所有组件内部的嵌套线框**：`PixelTextInput` / `PixelTextArea` / `PixelComboBox` / `PixelSelect` / `PixelNumberInput` / `PixelAutoComplete` / `PixelTagInput` / `PixelSearchBox` / `PixelMention` / `PixelCascader` / `PixelDatePicker` / `PixelTreeSelect` 等输入类；复选 / 开关 / 标签 / 徽章 / 评分 / 色块 / 分页 / 步骤等小件；以及任何「框里再套框」的内层 |

一句话规则：**组件的外框统一 2px，框里再套的框统一 1px；输入类与小件形体小，自己的框也用 1px**，这样同一屏不会再出现「有的组件比邻居粗/细」。

```slint
PixelTheme.border-width = 1px;
PixelTheme.primary-border-width = 1px;
PixelTheme.border-thin = 1px;   // 全 1px：最轻
// 或全 2px：保留像素分量感
```

`PixelPresets.classic()` / `soft()` / `dark()` 内置都是 2px / 2px / 1px（同一套两级体系）。
`cargo test -p slint-pixel --test theme_radius` 里的 `every_border_width_is_theme_driven`
盯住"组件里不许再出现硬编码宽度"，防止又冒出改不动的第四档（标题栏字形框、滑块轨道两处内部细节例外，已在测试里列明理由）。

## 组件清单

### 基础控件

| 组件 | 关键属性 | 回调 |
| --- | --- | --- |
| `PixelButton` | `text` / `variant` / `size` / `active` | `clicked` |
| `PixelCheckBox` | `checked` / `text` / `enabled` | `toggled` |
| `PixelSwitch` | `checked` / `enabled` | `toggled` |
| `PixelSlider` | `value` / `min` / `max` | `changed(value)` |
| `PixelTextInput` | `text` / `placeholder` | `accepted` / `edited` |
| `PixelProgressBar` | `progress` | — |
| `PixelBadge` | `text` | — |
| `PixelPanel` / `PixelDialog` | `title` / `open` | `accept` / `cancel` |
| `PixelRadioButton` / `PixelRadioGroup` | `checked` / `options` / `index` | `toggled` / `selected` |
| `PixelComboBox` / `PixelMenu` | `model` / `items` / `open` | `selected(index)` |
| `PixelTableView` | `headers` / `rows` / `selected-row` | `row-clicked(row)` |
| `PixelScrollPanel` | — | — |
| `PixelCollapsible` / `PixelAccordion` | `title` / `expanded` / `titles` | `toggled` / `toggled(index)` |
| `PixelSidebar` | `items` / `active` | `selected(index)` |

### 文本 / 表单

| 组件 | 关键属性 | 回调 |
| --- | --- | --- |
| `PixelText` / `PixelTitle` | `text` | — |
| `PixelForm` / `PixelFormItem` | `label` | — |
| `PixelTextArea` | `text` / `placeholder` | `accepted` / `edited` |
| `PixelIconButton` | `icon` / `active` | `clicked` |
| `PixelTooltip` / `PixelBubble` / `PixelPopconfirm` | `text` / `open` | `accept` / `cancel` |

### 反馈 / 展示

| 组件 | 关键属性 | 回调 |
| --- | --- | --- |
| `PixelAlert` | `status` / `text` | `closed` |
| `PixelToast` | `text` / `open` | `closed` |
| `PixelSpinner` / `PixelSkeleton` | — | — |
| `PixelTabs` / `PixelBreadcrumb` / `PixelPagination` | `tabs` / `items` / `page` | `selected` / `changed` |
| `PixelDrawer` / `PixelAvatar` / `PixelTag` | `open` / `text` / `closable` | `close` |
| `PixelCard` / `PixelNavbar` / `PixelEmpty` / `PixelDivider` / `PixelStat` | `title` / `items` / `value` | — |

### ROUND 2 扩展

| 组件 | 关键属性 | 回调 |
| --- | --- | --- |
| `PixelSegmentedControl` | `options` / `index` | `selected(index)` |
| `PixelSteps` | `titles` / `current` | `step-clicked(index)` |
| `PixelNumberInput` | `value` / `min` / `max` / `step` | `changed(value)` |
| `PixelSelect` | `options` / `value` / `open` | `changed(value)` |
| `PixelSwatchGroup` / `PixelColorPicker` | `colors` / `value` | `selected` / `changed` |
| `PixelContextMenu` | `items` / `open` | `selected(index)` |
| `PixelRangeSlider` | `lower` / `upper` / `min` / `max` | `changed(lower, upper)` |
| `PixelTree` / `PixelTreeSelect` | `items` / `levels` | `item-selected` |
| `PixelTransfer` / `PixelTransferPro` | `left-items` / `right-items` | `move-to-*` |
| `PixelUpload` | `file-name` / `hint` | `browse` / `clear` |
| `PixelTimeline` / `PixelCarousel` / `PixelSplitPane` | `items` / `slides` / `ratio` | `selected` / `changed` |
| `PixelDatePicker` / `PixelTimePicker` | `year` / `month` / `day` / `hour` / `minute` | `changed` |
| `PixelCommandPalette` / `PixelKanban` / `PixelOnboarding` | `items` / `columns` / `titles` | `selected` / `finished` |
| `PixelImage` / `PixelImageViewer` / `PixelImageCrop` | `source` / `zoom` | `clicked` / `crop` |
| `PixelTagInput` / `PixelAutoComplete` / `PixelCascader` | `tags` / `options` / `level1` / `level2` | `add-tag` / `selected` |
| `PixelDateRangePicker` | `start-*` / `end-*` | `changed(...)` |
| `PixelRating` | `value` / `max` | `changed(value)` |
| `PixelDescriptionList` / `PixelResult` | `labels` / `values` / `status` | `action` |
| `PixelAvatarGroup` / `PixelList` / `PixelVirtualList` | `names` / `items` | `item-selected` |
| `PixelTreeTable` / `PixelDataTable` | `headers` / `rows` | `row-clicked` |
| `PixelSearchBox` / `PixelNotificationCenter` | `query` / `messages` | `search` / `dismiss` |
| `PixelSubMenu` | `items` / `children` | `selected(parent, child)` |
| `PixelBackTop` / `PixelAffix` / `PixelSticky` | — | `clicked` |
| `PixelRichTextEditor` / `PixelCodeBlock` | `value` / `code` | `copy` |
| `PixelQRCode` / `PixelWatermark` / `PixelCalendar` | `code` / `text` / `year` | `date-changed` |

### ROUND 3 高级

| 组件 | 关键属性 | 回调 |
| --- | --- | --- |
| `PixelChart` / `PixelSparkline` | `data` / `labels` | `bar-clicked(index)` |
| `PixelDataGrid` / `PixelProTable` | `headers` / `rows` / `column-widths` | `sort-requested` / `column-resize` |
| `PixelVirtualScroll` | `items` / `row-height` | `item-selected` |
| `PixelTreePro` | `items` / `levels` / `expanded` | `item-selected` / `toggle` |
| `PixelFormValidator` | `fields` / `valid` / `errors` | `submit` |
| `PixelScheduler` / `PixelCalendarAgenda` | `times` / `events` / `dates` | `event-clicked` |
| `PixelWizard` / `PixelMultiStepForm` | `steps` / `current` | `previous` / `next` / `finish` |
| `PixelKanbanPro` | `columns` / `cards` | `drag-start` / `drag-over` / `drop` |
| `PixelCommandPalettePro` | `groups` / `commands` / `group-index` | `selected` |
| `PixelColorPickerPro` | `value` / `hex` | `changed` / `hex-changed` |
| `PixelMention` / `PixelOTPInput` / `PixelSignature` | `options` / `value` / `length` | `selected` / `changed` / `begin` / `move` / `end` |
| `PixelDrawerPro` | `open` / `drawer-width` | `close` |
| `PixelOrgChart` / `PixelFlowChart` / `PixelMindMap` | `names` / `nodes` / `center-text` | `item-selected` |
| `PixelGantt` / `PixelMap` / `PixelGeo` | `tasks` / `markers` | — |
| `PixelVideoPlayer` / `PixelAudioPlayer` | `playing` / `progress` | `play` / `pause` / `seek` |
| `PixelPDFViewer` / `PixelPrintPreview` | `page` / `pages` | `page-changed` / `print` |
| `PixelBarcode` / `PixelCaptcha` | `code` | `refresh` |

## 示例：PixelDataGrid 列宽与排序

```slint
PixelDataGrid {
    headers: ["名称", "数值"];
    rows: [["Alpha", "10"], ["Beta", "20"]];
    column-widths: [120px, 80px];
    sort-requested(column) => { /* Rust 侧排序后更新 rows */ }
    column-resize(column, width) => { /* 更新 column-widths[column] */ }
}
```

## 示例：PixelFormValidator

```slint
PixelFormValidator {
    fields: ["昵称", "邮箱"];
    valid: [true, false];
    errors: ["", "邮箱格式不正确"];
    submit => { /* 校验通过后提交 */ }
}
```

## 示例：PixelChart

```slint
PixelChart {
    data: [40.0, 80.0, 55.0, 90.0, 70.0];
    labels: ["A", "B", "C", "D", "E"];
    bar-clicked(index) => { /* 点击柱状图 */ }
}
```

## 测试

```bash
cargo test --workspace
```

`slint-pixel` 内置：

- `canvas.rs` 单元测试（画布绘制 / PNG 导出）
- `tests/ui_smoke.rs` 全组件 compile smoke 测试（把 `@slint_pixel` 全部组件编译进一个 Window）
