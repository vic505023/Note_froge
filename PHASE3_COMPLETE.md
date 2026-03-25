# Фаза 3 — AI-интеграция и настройки (ЗАВЕРШЕНА)

## Что реализовано

### Backend (Rust) ✅

1. **AI HTTP-клиент** (`src-tauri/src/services/ai_client.rs`)
   - Стриминг через `chat_stream()`
   - Полный ответ через `chat_complete()`
   - Тест соединения `test_connection()`
   - Обработка SSE (Server-Sent Events)
   - Таймауты и graceful degradation

2. **AI Commands** (`src-tauri/src/commands/ai.rs`)
   - `ai_chat` — чат с AI со стримингом
   - `ai_test_connection` — проверка API
   - `ai_edit_note` — AI-редактирование заметки
   - `save_chat_message` — сохранение истории
   - `get_chat_history` — загрузка истории
   - `clear_chat_history` — очистка истории

3. **Settings Commands** (`src-tauri/src/commands/settings.rs`)
   - `get_settings` — чтение настроек
   - `update_settings` — сохранение настроек в config.toml

### Frontend (Svelte) ✅

1. **Новые компоненты**
   - `Settings.svelte` — полноценный UI настроек
   - `ChatTab.svelte` — чат с AI
   - `ChatMessage.svelte` — рендеринг сообщения (markdown)
   - `DiffView.svelte` — отображение изменений с кнопками Apply/Reject

2. **Новые stores**
   - `ai.ts` — состояние AI (сообщения, стриминг, edit mode)

3. **Новые утилиты**
   - `diff.ts` — построчное сравнение текста
   - Обновлён `tauri.ts` — все AI commands

4. **Обновлённые компоненты**
   - `AIPanel.svelte` — теперь использует `ChatTab`
   - `Layout.svelte` — добавлены Settings и горячие клавиши

## Горячие клавиши

| Клавиша | Действие |
|---------|----------|
| **Ctrl+,** | Открыть настройки |
| **Ctrl+Enter** | Отправить сообщение в чате |
| **Escape** | Закрыть настройки / остановить AI-стриминг |

## Настройки

### Vault
- Путь к vault с кнопкой Browse
- Кнопка "Reindex vault" для полной переиндексации

### AI
- Base URL (по умолчанию `https://api.ranvik.ru/v1`)
- API Key (маскируется как `sk-...xxxx`)
- Model (по умолчанию `gpt-4o`)
- Embedding Model (по умолчанию `text-embedding-3-small`)
- Кнопка "Test Connection" с индикацией результата

### Editor
- Font Family (по умолчанию `JetBrains Mono`)
- Font Size (10-24px, по умолчанию 14px)
- Autosave Delay (500-10000ms, по умолчанию 1000ms)

### Appearance
- Theme (пока только Dark)
- Sidebar Width (150-400px, слайдер)
- AI Panel Width (250-600px, слайдер)

## Функции AI-чата

### Режим чата (Chat Mode)
- Контекст текущей заметки автоматически передаётся AI
- Markdown рендеринг в ответах AI
- Стриминг ответов в реальном времени
- История сохраняется в SQLite (привязана к заметке)

### Режим редактирования (Edit Mode)
- Toggle кнопкой "Edit" в toolbar
- AI возвращает полный новый текст заметки
- Отображается diff с подсветкой изменений
- Кнопки Apply / Reject для применения/отклонения

### Автоматическое редактирование через `<noteforge-edit>`
- Если AI в режиме Chat возвращает `<noteforge-edit>...</noteforge-edit>`, автоматически показывается diff
- Пользователь может применить или отклонить изменения

## Как тестировать

### 1. Настройка API

```bash
cargo tauri dev
```

1. Откройте настройки (Ctrl+,)
2. Введите API key и Base URL
3. Нажмите "Test Connection"
4. Сохраните (Save)

### 2. Тест чата

1. Откройте любую заметку
2. В правой панели (AI Panel) выберите вкладку "Chat"
3. Введите вопрос: "Summarize this note"
4. Наблюдайте стриминг ответа

### 3. Тест редактирования (Edit Mode)

1. В чате нажмите кнопку "Edit"
2. Введите инструкцию: "Add a section about testing"
3. Откроется diff-вью с изменениями
4. Нажмите Apply для применения

### 4. Тест автоматического редактирования

1. Вернитесь в режим Chat
2. Напишите: "Rewrite this note in a more formal tone"
3. AI вернёт текст в тегах `<noteforge-edit>`
4. Появится diff-вью с кнопками Apply/Reject

## Graceful Degradation

- Если API key не указан → показывается сообщение с инструкцией открыть Settings
- Если API недоступен → ошибка отображается в чате, приложение не крашится
- Если стриминг оборвался → показывается то что успело прийти + сообщение об ошибке
- Если заметка не открыта → input отключён с placeholder "Open a note to start chatting"

## История чата

- Сохраняется в SQLite (таблица `chat_history`)
- Привязана к заметке (`note_path`)
- Загружается автоматически при открытии заметки
- Кнопка "Clear" для очистки истории текущей заметки
- Режим чата (`mode = 'chat'`) отдельно от режима поиска (Phase 4)

## Известные ограничения

1. **Светлая тема** — заглушка "Coming Soon" (Phase 5)
2. **RAG Search** — вкладка Search заглушена (Phase 4)
3. **Stream cancellation** — кнопка Stop есть, но фактическая отмена запроса не реализована (API ограничение)
4. **Tab size** — поле убрано из Settings, так как не реализовано в EditorConfig

## Архитектура

```
User Input
    ↓
ChatTab.svelte → aiStore.sendMessage()
    ↓
invoke('ai_chat', { messages, note_context })
    ↓
Rust: ai_chat command
    ↓
AiClient.chat_stream()
    ↓
API Request (OpenAI-compatible)
    ↓
SSE Stream → parse chunks → emit 'ai-chunk'
    ↓
Frontend: listen('ai-chunk') → update currentStreamContent
    ↓
Render ChatMessage with streaming cursor
    ↓
'ai-done' → move to messages, save to SQLite
```

## Следующие фазы

- **Фаза 4**: RAG-поиск (embeddings, semantic search)
- **Фаза 5**: Полировка (светлая тема, вкладки, file watcher)
