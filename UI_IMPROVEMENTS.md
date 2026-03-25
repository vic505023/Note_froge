# Улучшения интерфейса NoteForge — Единый Header

## ✅ Главная проблема решена

**Было:** Три разрозненных header блока (Notes | Example.md | AI Assistant)
**Стало:** Единый header bar на весь экран

---

## 🎯 Что исправлено

### 1. **Создан единый Header компонент** (`Header.svelte`)

**Структура:**
```
[Notes] [+] [←]  │  [📄 Example.md]  │  [AI Assistant] [→]
```

**Что решает:**
- ✅ Все три секции выровнены по одной высоте (48px)
- ✅ Единый фон (`--bg-secondary`)
- ✅ Аккуратные разделители между секциями (`border-soft`)
- ✅ Вертикальное центрирование всех элементов
- ✅ Консистентные отступы (padding: 16px)

**Логика:**
- Sidebar section: показывает "Notes" + кнопки или toggle кнопку
- Editor section: показывает имя файла с иконкой (центрировано)
- AI section: показывает "AI Assistant" + кнопку или toggle кнопку

---

### 2. **Название заметки (Example.md)** — убран button-стиль

**Было:**
- Выглядело как кнопка/badge с фоном и border
- Было в отдельном tab-bar

**Стало:**
- Обычный текст в header
- Иконка документа + название
- Без фона, без рамок
- Центрировано в editor section

**Стили:**
```css
.current-file {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-primary);
  font-size: 0.875rem;
  font-weight: 500;
}
```

---

### 3. **Toggle кнопки** — больше не накладываются

**Было:**
- Абсолютное позиционирование (top: 12px, left/right: 12px)
- Накладывались на название файла
- z-index конфликты

**Стало:**
- Интегрированы в header sections
- Когда sidebar закрыт — кнопка toggle в sidebar section
- Когда AI panel закрыт — кнопка toggle в AI section
- Корректный layout без наложений

**CSS:**
```css
.sidebar-section:has(.toggle-btn),
.ai-section:has(.toggle-btn) {
  width: auto;
  min-width: 48px;
  justify-content: center;
}
```

---

### 4. **Layout.svelte** — переработан

**Изменения:**
- Добавлен `<Header />` компонент сверху
- Убраны отдельные toggle кнопки
- `main-content` — flex контейнер для панелей
- Структура:
  ```
  layout (flex-column)
    ├─ Header (48px)
    └─ main-content (flex: 1)
        ├─ sidebar (260px)
        ├─ editor (flex: 1)
        └─ ai-panel (360px)
  ```

---

### 5. **Sidebar.svelte** — убран header

**Было:**
```svelte
<div class="sidebar-header">
  <h2>Notes</h2>
  <buttons>...</buttons>
</div>
```

**Стало:**
- Только `file-list` с `<FileTree />`
- Header перенесен в общий Header компонент
- Чище, без дублирования

---

### 6. **EditorPane.svelte** — убран tab-bar

**Было:**
```svelte
<div class="tab-bar">
  <div class="tab">📄 Example.md</div>
</div>
```

**Стало:**
- Убран tab-bar полностью
- Название файла теперь в Header
- Сразу начинается `editor-wrapper`

**Улучшения:**
- `max-width: 880px` (было 820px) — чуть больше пространства
- `padding: 0 48px` (было 24px 32px) — больше воздуха по бокам
- `padding-top: 32px` — отступ сверху

---

### 7. **AIPanel.svelte** — убран header

**Было:**
```svelte
<div class="panel-header">
  <h2>AI Assistant</h2>
  <button>→</button>
</div>
```

**Стало:**
- Только tabs и content
- Header перенесен в общий Header
- Tabs улучшены (больше padding, gap)

---

### 8. **Editor.svelte** — улучшена типографика

**Изменения:**

**Шрифт:**
- Было: `'JetBrains Mono'` (моноширинный)
- Стало: `'Inter', system-ui` (пропорциональный, читаемый)

**Размеры:**
- font-size: 15px (было 14px)
- line-height: 1.75 (было 1.7)

**Заголовки:**
- H1: 2rem, font-weight: 700, letter-spacing: -0.02em
- H2: 1.5rem, font-weight: 600, letter-spacing: -0.01em
- H3: 1.25rem, font-weight: 600

**Padding:**
- `.cm-content`: padding: 16px 0 (было 8px)
- `.cm-line`: padding: 0 8px (было 0 4px)

**Результат:**
- Лучшая читаемость
- Четкая иерархия заголовков
- Больше воздуха между строками

---

## 📐 Технические детали

### Header секции

**Sidebar section (260px):**
```css
.sidebar-section {
  width: 260px;
  justify-content: space-between;
  border-right: 1px solid var(--border-soft);
}
```

**Editor section (flex: 1):**
```css
.editor-section {
  flex: 1;
  justify-content: center; /* центрирует имя файла */
}
```

**AI section (360px):**
```css
.ai-section {
  width: 360px;
  justify-content: space-between;
  border-left: 1px solid var(--border-soft);
}
```

### Адаптивные ширины

Когда панель закрыта:
```css
.sidebar-section:has(.toggle-btn) {
  width: auto;
  min-width: 48px;
}
```

---

## 🎨 Визуальные улучшения

### Единый фон
- Весь header: `--bg-secondary`
- Мягкие разделители: `--border-soft`

### Консистентные отступы
- Padding sections: 16px
- Gap между элементами: 8px
- Иконочные кнопки: 28x28px

### Типографика
- Section titles: 0.8125rem, uppercase, letter-spacing
- File name: 0.875rem, font-weight: 500
- Иконки: 14-16px

---

## 🚀 Что улучшилось

1. ✅ **Единый header** — нет ощущения "трёх разных панелей"
2. ✅ **Название файла** — теперь обычный текст, не button
3. ✅ **Toggle кнопки** — больше не накладываются на контент
4. ✅ **Выравнивание** — все секции одной высоты, вертикально центрированы
5. ✅ **Читаемость** — улучшена типографика, line-height, padding
6. ✅ **Воздух** — больше пространства в editor (max-width: 880px, padding: 48px)
7. ✅ **Минимализм** — убраны лишние borders, backgrounds, button-стили

---

## 📦 Изменённые файлы

1. **Header.svelte** (новый) — единый header bar
2. **Layout.svelte** — добавлен Header, убраны toggle кнопки
3. **Sidebar.svelte** — убран header, только file-list
4. **EditorPane.svelte** — убран tab-bar, улучшены отступы
5. **AIPanel.svelte** — убран header, улучшены tabs
6. **Editor.svelte** — улучшена типографика (шрифт, размеры, заголовки)

---

## 🧪 Как проверить

```bash
npm run tauri dev
```

**Что смотреть:**
1. Header — одна высота, три секции выровнены
2. Название файла — центрировано, обычный текст
3. Toggle кнопки — встроены в header, не накладываются
4. Sidebar/AI panel — без собственных headers
5. Editor — больше воздуха, лучше читается

---

## 🎯 Осталось (опционально)

- Command palette (⌘K)
- Breadcrumbs для вложенных файлов
- Индикатор сохранения
- Множественные вкладки
- Контекстное меню

---

Архитектура и логика не тронуты — только UI/layout.
