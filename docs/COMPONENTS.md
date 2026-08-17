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
