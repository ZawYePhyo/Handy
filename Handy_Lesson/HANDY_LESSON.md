# Handy Project: A Deep Dive Into Desktop App Development

*A journey through forking, customizing, and learning from a production-ready Tauri application*

---

## Table of Contents
1. [[#What We Built]]
2. [[#The Architecture: How It All Fits Together]]
3. [[#The Technology Stack: Why These Choices Matter]]
4. [[#Codebase Structure: Finding Your Way Around]]
5. [[#Our Custom Features: Edit and Translate]]
6. [[#The Debugging Journey: What Went Wrong and How We Fixed It]]
7. [[#Working With a Fork: Managing Upstream Changes]]
8. [[#Lessons Learned: Wisdom From The Trenches]]
9. [[#Best Practices: How Good Engineers Think]]
10. [[#What's Next: Future Considerations]]

---

## What We Built

### The Big Picture

Imagine you're at a restaurant. The original chef (CJ Pais) created an amazing recipe for speech-to-text software called **Handy**. It's open source, which means the recipe is freely available for anyone to use and modify. We took that recipe, made a copy (forked it), and added our own special ingredients: an edit button and a translation feature for Burmese-to-English.

Handy is a **desktop application** that converts your speech into text. Press a hotkey, speak, and your words appear in any text field. The magic? It all happens on your computer—no cloud, no privacy concerns, no internet required (well, except for our Gemini translation feature).

### What Makes This Interesting

This isn't a toy project. Handy is a **production-grade application** with:
- **715 Rust dependencies** (we're standing on the shoulders of giants)
- **Support for 14 languages**
- **Cross-platform compatibility** (Windows, macOS, Linux)
- **AI integration** (Whisper models, Parakeet, and Gemini)
- **Complex audio processing** (Voice Activity Detection, resampling, device management)
- **Real-time performance requirements** (nobody wants laggy speech-to-text)

When you fork a project like this, you're not starting from scratch—you're learning from thousands of hours of engineering decisions.

---

## The Architecture: How It All Fits Together

### The Restaurant Analogy

Think of Handy like a restaurant with two distinct areas:

**The Kitchen (Rust Backend)**
- Does the heavy lifting: audio processing, AI inference, database operations
- Fast, efficient, handles raw ingredients (audio bytes, file I/O)
- Written in Rust for performance and safety
- Like a professional kitchen with industrial equipment

**The Dining Room (React Frontend)**
- Beautiful, user-friendly interface
- Displays menus (settings), takes orders (user interactions)
- Written in React/TypeScript for quick UI iteration
- Like a well-designed dining space customers interact with

**The Waiter (Tauri Bridge)**
- Connects the kitchen and dining room
- Takes orders from the frontend, delivers results from the backend
- Uses a typed command/event system
- Like a waiter taking orders and bringing food

### The Real Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     FRONTEND (React)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Settings UI │  │  History UI  │  │  Model UI    │  │
│  │  (Components)│  │  (Components)│  │  (Components)│  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                  │                  │          │
│         └──────────────────┼──────────────────┘          │
│                            ▼                             │
│                  ┌──────────────────┐                    │
│                  │  Zustand Stores  │                    │
│                  │  (State Mgmt)    │                    │
│                  └────────┬─────────┘                    │
│                           ▼                              │
│                  ┌──────────────────┐                    │
│                  │  Tauri Commands  │ ◄─── bindings.ts  │
│                  │  (Auto-generated)│                    │
└──────────────────┴────────┬─────────┴────────────────────┘
                            │
                   ═════════╪══════════ Tauri Bridge
                            │
┌───────────────────────────┴──────────────────────────────┐
│                   BACKEND (Rust)                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Command Handlers                     │   │
│  │  (commands/history.rs, commands/audio.rs, etc.)  │   │
│  └────────────────────┬─────────────────────────────┘   │
│                       ▼                                  │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Manager Layer                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │   │
│  │  │   History   │  │    Audio    │  │  Model   │ │   │
│  │  │   Manager   │  │   Manager   │  │ Manager  │ │   │
│  │  └──────┬──────┘  └──────┬──────┘  └────┬─────┘ │   │
│  └─────────┼─────────────────┼──────────────┼───────┘   │
│            │                 │              │            │
│            ▼                 ▼              ▼            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   SQLite     │  │  Audio I/O   │  │  AI Models   │  │
│  │  Database    │  │  (CPAL)      │  │  (Whisper)   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### Key Architectural Patterns

**1. The Manager Pattern**

Instead of scattering logic everywhere, Handy uses "managers" - specialized objects that handle one domain:

```rust
// Each manager is like a department head in a company
pub struct HistoryManager {
    app_handle: AppHandle,      // Connection to the app
    recordings_dir: PathBuf,     // Where audio files live
    db_path: PathBuf,           // Where the database lives
}

// They own their resources and know how to use them
impl HistoryManager {
    pub fn save_transcription(...) { /* ... */ }
    pub fn get_history_entries(...) { /* ... */ }
    pub fn cleanup_old_entries(...) { /* ... */ }
}
```

**Why this matters**: When you want to add a feature (like we did with edit/translate), you know exactly where to look. Need to update a transcription? Go to HistoryManager. Need audio devices? Go to AudioManager. It's like knowing which department handles what in a company.

**2. Command-Event Architecture**

The frontend and backend communicate through two channels:

**Commands** (Frontend → Backend): "Please do this for me"
```typescript
// Frontend asks politely
await commands.updateHistoryEntryText(id, newText);
```

**Events** (Backend → Frontend): "Hey, something changed!"
```typescript
// Frontend listens for news
listen("history-updated", () => {
    loadHistoryEntries(); // Refresh the UI
});
```

This is like a waiter (commands) taking your order to the kitchen, and a bell (events) ringing when your food is ready. You don't watch the kitchen cook; you wait for the signal.

**3. Type Safety Across The Bridge**

Here's where it gets magical. Tauri uses a tool called **tauri-specta** to automatically generate TypeScript types from Rust:

```rust
// Backend (Rust)
#[tauri::command]
#[specta::specta]  // ← This magic annotation
pub async fn update_history_entry_text(
    id: i64,
    new_text: String,
) -> Result<(), String> {
    // Implementation...
}
```

This automatically creates:
```typescript
// Frontend (bindings.ts - auto-generated!)
async updateHistoryEntryText(
    id: number,
    newText: string
): Promise<Result<null, string>> {
    // Implementation generated...
}
```

**The lesson**: You get compile-time safety across languages! If you change the Rust function signature, your TypeScript code will fail to compile. No more runtime errors from mismatched types.

---

## The Technology Stack: Why These Choices Matter

### Tauri: The Game Changer

**What it is**: A framework for building desktop apps using web technologies (HTML/CSS/JS) for the UI and Rust for the backend.

**Why not Electron?**
- **Size**: Electron bundles an entire Chromium browser (~100-200MB). Tauri uses the OS's built-in webview (~3-10MB).
- **Memory**: Electron loads a whole browser instance. Tauri shares the system webview.
- **Security**: Tauri's Rust backend is memory-safe by default. No more buffer overflows.
- **Performance**: Rust is *fast*. Like, really fast. Perfect for audio processing.

**The tradeoff**: Tauri is newer, so the ecosystem is smaller. But for an app like Handy that needs performance and small size, it's the right choice.

### Rust: Why The Backend Language Matters

You might think: "Why not just use JavaScript everywhere?"

Here's why Rust wins for Handy's backend:

**1. Performance**
```rust
// Processing 16,000 audio samples per second
for &sample in samples.iter() {
    let sample_i16 = (sample * i16::MAX as f32) as i16;
    writer.write_sample(sample_i16)?;
}
```

This runs in nanoseconds. In JavaScript, you'd be waiting milliseconds. When you're processing real-time audio, milliseconds are forever.

**2. Memory Safety Without Garbage Collection**

JavaScript pauses to collect garbage. Rust doesn't have garbage collection—it knows at compile time when to free memory:

```rust
{
    let audio_buffer = vec![0.0; 16000]; // Allocate
    process_audio(&audio_buffer);        // Use
} // ← audio_buffer automatically freed here, no pauses
```

**3. Fearless Concurrency**

Rust prevents data races at compile time:
```rust
// This won't compile if it's unsafe:
let shared_data = Arc::new(Mutex::new(data));
thread::spawn(move || {
    let mut data = shared_data.lock().unwrap();
    data.push(value); // Safe!
});
```

In JavaScript, you'd get runtime race conditions. In Rust, the compiler is your guardian angel.

### React + TypeScript: The UI Layer

**Why React?**
- **Component model**: Build UI like LEGO blocks
- **Huge ecosystem**: Need a date picker? npm install. Need charts? npm install.
- **Fast iteration**: Change code, see results instantly (hot reload)

**Why TypeScript?**
```typescript
// TypeScript catches this at compile time:
const entry: HistoryEntry = {
    id: 123,
    transcription_text: "Hello",
    // Missing required fields! TypeScript error!
};

// JavaScript would explode at runtime when you try to access
// entry.timestamp (undefined)
```

**The pattern**: Rust for the engine, React for the dashboard. Rust ensures the car runs perfectly; React makes sure the driver has a nice interface.

### The AI Stack: Whisper, Gemini, and ONNX Runtime

**Whisper.cpp**: OpenAI's speech recognition model, compiled to C++ for speed
- **Why C++?**: Python is too slow for real-time audio. C++ is close to the metal.
- **ONNX Runtime**: Runs AI models efficiently, supports GPU acceleration
- **The integration**: Rust → FFI bindings → C++ → GPU → AI magic

**Gemini API**: Google's language model for translation
- **Why not local?**: Translation models are HUGE (10+ GB). Not practical for desktop.
- **Tradeoff**: Requires internet, but translation isn't real-time critical like transcription

### SQLite: The Unsung Hero

For storing transcription history:
```sql
CREATE TABLE transcription_history (
    id INTEGER PRIMARY KEY,
    transcription_text TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    saved BOOLEAN DEFAULT 0
);
```

**Why SQLite?**
- **No server needed**: It's just a file (`history.db`)
- **ACID guarantees**: Your data won't corrupt
- **Fast**: Reads happen in microseconds
- **Migrations**: Built-in versioning (we use `rusqlite_migration`)

---

## Codebase Structure: Finding Your Way Around

### The Map

```
Handy/
├── src/                          # Frontend (React/TypeScript)
│   ├── components/
│   │   ├── settings/
│   │   │   ├── history/
│   │   │   │   └── HistorySettings.tsx    # ← Our custom edit/translate UI
│   │   │   ├── general/
│   │   │   │   └── GeminiApiKey.tsx       # ← API key with save button
│   │   │   └── ...
│   │   ├── model-selector/
│   │   └── ui/                  # Reusable components (Button, Input, etc.)
│   ├── hooks/                   # Custom React hooks
│   │   └── useSettings.ts       # Settings state management
│   ├── stores/                  # Zustand state stores
│   │   └── settingsStore.ts     # Global settings
│   ├── i18n/                    # Internationalization (14 languages!)
│   ├── bindings.ts              # ← AUTO-GENERATED (never edit manually!)
│   └── App.tsx                  # Main app component
│
├── src-tauri/                   # Backend (Rust)
│   ├── src/
│   │   ├── commands/            # Command handlers (Frontend → Backend)
│   │   │   ├── history.rs       # ← Our custom edit/translate commands
│   │   │   ├── audio.rs
│   │   │   └── ...
│   │   ├── managers/            # Business logic layer
│   │   │   ├── history.rs       # ← We added update_transcription_text()
│   │   │   ├── audio.rs
│   │   │   ├── model.rs
│   │   │   └── transcription.rs
│   │   ├── audio_toolkit/       # Low-level audio processing
│   │   ├── gemini_client.rs     # ← We added translate_text()
│   │   ├── settings.rs          # App settings management
│   │   ├── lib.rs              # App entry point, command registration
│   │   └── main.rs             # Just boots lib.rs
│   ├── resources/              # Bundled resources (models, sounds)
│   ├── tauri.conf.json         # App configuration
│   └── Cargo.toml              # Rust dependencies
│
├── CLAUDE.md                   # Developer guide (for Claude Code)
├── BUILD.md                    # Build instructions
├── LICENSE                     # MIT License (original author: CJ Pais)
└── README.md                   # Project overview
```

### Key Files Explained

**`src/bindings.ts`** - The Rosetta Stone
```typescript
// AUTO-GENERATED by tauri-specta
// Maps Rust types to TypeScript types
export type HistoryEntry = {
    id: number;
    transcription_text: string;
    timestamp: number;
    // ... more fields
};

export const commands = {
    async updateHistoryEntryText(id: number, newText: string) {
        // Generated IPC code
    }
};
```

**Never edit this file manually!** It's regenerated when you run `cargo build` in debug mode.

**`src-tauri/src/lib.rs`** - The Backend Registry
```rust
// This is where you register new commands
let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
        // ... 80+ commands
        commands::history::update_history_entry_text,  // ← We added this
        commands::history::translate_history_entry,     // ← And this
    ]);
```

**`src-tauri/tauri.conf.json`** - The App's Identity
```json
{
    "identifier": "com.pais.handy",  // ← Original author's namespace
    "productName": "Handy",
    "version": "0.7.0"
}
```

### Navigation Strategy

When adding a feature:
1. **Start with the UI**: Where does the user see this? (`src/components/`)
2. **Add a command**: How does the UI talk to the backend? (`src-tauri/src/commands/`)
3. **Implement business logic**: What actually happens? (`src-tauri/src/managers/`)
4. **Register the command**: Make it callable (`src-tauri/src/lib.rs`)
5. **Rebuild to generate bindings**: `cargo build` (debug mode)
6. **Use it in the frontend**: Import from `bindings.ts`

---

## Our Custom Features: Edit and Translate

### Feature 1: Edit Transcription Text

**The Problem**: Gemini struggles with Burmese words sometimes. Users need to manually correct transcriptions.

**The Solution**: Add a Pencil icon button that makes the text editable.

#### Backend Implementation

**Step 1: Create the Command** (`src-tauri/src/commands/history.rs`)
```rust
#[tauri::command]
#[specta::specta]  // ← Marks for TypeScript generation
pub async fn update_history_entry_text(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    new_text: String,
) -> Result<(), String> {
    history_manager
        .update_transcription_text(id, new_text)
        .await
        .map_err(|e| e.to_string())
}
```

**Key decisions**:
- `State<'_, Arc<HistoryManager>>`: Tauri injects the manager automatically
- `Result<(), String>`: Returns nothing on success, error message on failure
- `async`: Non-blocking, UI stays responsive

**Step 2: Implement Business Logic** (`src-tauri/src/managers/history.rs`)
```rust
pub async fn update_transcription_text(
    &self,
    id: i64,
    new_text: String
) -> Result<()> {
    let conn = self.get_connection()?;

    // Update the database
    conn.execute(
        "UPDATE transcription_history SET transcription_text = ?1 WHERE id = ?2",
        params![new_text, id],
    )?;

    debug!("Updated transcription text for entry {}", id);

    // ← CRITICAL: Notify the frontend!
    if let Err(e) = self.app_handle.emit("history-updated", ()) {
        error!("Failed to emit history-updated event: {}", e);
    }

    Ok(())
}
```

**The event pattern**: The backend emits `history-updated`, and the frontend's event listener automatically refreshes. This is **reactive programming**—change propagates automatically.

**Step 3: Register** (`src-tauri/src/lib.rs`)
```rust
.commands(tauri_specta::collect_commands![
    // ...
    commands::history::update_history_entry_text,  // ← Add this line
    // ...
])
```

#### Frontend Implementation

**Step 1: Add Edit State** (`src/components/settings/history/HistorySettings.tsx`)
```typescript
const [isEditing, setIsEditing] = useState(false);
const [editedText, setEditedText] = useState(entry.transcription_text);
const [isSaving, setIsSaving] = useState(false);
```

**Step 2: Handle Editing**
```typescript
const handleStartEdit = () => {
    setIsEditing(true);
    setEditedText(entry.transcription_text);
};

const handleSaveEdit = async () => {
    setIsSaving(true);
    try {
        await commands.updateHistoryEntryText(entry.id, editedText);
        setIsEditing(false);  // Exit edit mode on success
    } catch (error) {
        console.error("Failed to update:", error);
        alert("Failed to save changes. Please try again.");
    } finally {
        setIsSaving(false);
    }
};

const handleCancelEdit = () => {
    setIsEditing(false);
    setEditedText(entry.transcription_text);  // Revert changes
};
```

**Step 3: Conditional Rendering**
```typescript
{isEditing ? (
    <>
        <textarea
            value={editedText}
            onChange={(e) => setEditedText(e.target.value)}
            className="w-full p-2 text-sm rounded border"
            disabled={isSaving}
        />
        <button onClick={handleSaveEdit} disabled={isSaving}>
            <Save size={16} /> Save
        </button>
        <button onClick={handleCancelEdit} disabled={isSaving}>
            <X size={16} /> Cancel
        </button>
    </>
) : (
    <>
        <p>{entry.transcription_text}</p>
        <button onClick={handleStartEdit}>
            <Pencil size={16} /> Edit
        </button>
    </>
)}
```

**The pattern**: Optimistic UI updates. When you click Save, the UI assumes success and exits edit mode. If it fails, we show an error and let the user retry.

### Feature 2: Translate to English

**The Problem**: Burmese transcriptions need English translations for wider accessibility.

**The Solution**: Use the Gemini API (same key already configured) to translate on-demand.

#### Backend Implementation

**Step 1: Add Translation Function** (`src-tauri/src/gemini_client.rs`)
```rust
pub async fn translate_text(
    api_key: &str,
    text: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Gemini API key not configured".to_string());
    }

    let prompt = format!(
        "Translate the following text from Burmese to English. \
         Return only the translated text, nothing else:\n\n{}",
        text
    );

    // Try models in priority order
    let models = ["gemini-2.5-flash", "gemini-2.0-flash"];
    let mut last_error = String::new();

    for model in &models {
        match send_text_request(api_key, model, &prompt).await {
            Ok(translated) => return Ok(translated),
            Err(e) => {
                error!("Translation failed with {}: {}", model, e);
                last_error = e;
            }
        }
    }

    Err(format!("Translation failed: {}", last_error))
}
```

**Design decisions**:
- **Fallback models**: Try the best model first, fall back to older version
- **Simple prompt**: "Return only the translated text" prevents chattiness
- **Error propagation**: Detailed errors help debugging

**Step 2: HTTP Request Helper**
```rust
async fn send_text_request(
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part::Text {
                text: prompt.to_string(),
            }],
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    // Parse and extract the translation
    let gemini_response: GeminiResponse = response.json().await?;
    let text = gemini_response
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(text)
}
```

**Step 3: Create Command** (`src-tauri/src/commands/history.rs`)
```rust
#[tauri::command]
#[specta::specta]
pub async fn translate_history_entry(
    app: AppHandle,
    _history_manager: State<'_, Arc<HistoryManager>>,
    text: String,
) -> Result<String, String> {
    // Get API key from settings
    let settings = settings::get_settings(&app);
    let api_key = settings
        .post_process_api_keys
        .get("gemini_transcription")
        .ok_or_else(|| "Gemini API key not configured".to_string())?;

    // Call the translation function
    gemini_client::translate_text(api_key, &text).await
}
```

**The pattern**: Commands are thin wrappers. They fetch dependencies (API key) and call the real logic (in `gemini_client`).

#### Frontend Implementation

**Step 1: Translation State**
```typescript
const [isTranslating, setIsTranslating] = useState(false);
const [translatedText, setTranslatedText] = useState<string | null>(null);
```

**Step 2: Translation Handler**
```typescript
const handleTranslate = async () => {
    setIsTranslating(true);
    try {
        const result = await commands.translateHistoryEntry(
            entry.transcription_text
        );

        if (result.status === "ok") {
            setTranslatedText(result.data);
        } else {
            throw new Error(result.error || "Translation failed");
        }
    } catch (error) {
        console.error("Translation error:", error);
        alert(
            error instanceof Error
                ? error.message
                : "Failed to translate. Check your Gemini API key."
        );
    } finally {
        setIsTranslating(false);
    }
};
```

**Step 3: UI Display**
```typescript
<button
    onClick={handleTranslate}
    disabled={isTranslating}
    title={isTranslating ? "Translating..." : "Translate to English"}
>
    <Languages size={16} />
</button>

{translatedText && (
    <div className="mt-2 p-2 rounded bg-mid-gray/10 border-l-2 border-logo-primary">
        <p className="text-xs text-mid-gray uppercase mb-1">
            English Translation:
        </p>
        <p className="text-sm text-text/90">
            {translatedText}
        </p>
    </div>
)}
```

**The UX pattern**:
1. Button shows loading state (`isTranslating`)
2. Translation displays below the original (non-intrusive)
3. Errors show friendly alerts, not cryptic messages

---

## The Debugging Journey: What Went Wrong and How We Fixed It

### Bug #1: The API Key Vanishing Act

**What happened**: User enters Gemini API key, switches tabs, returns—API key is gone!

**The investigation**:
```typescript
// Original code
const handleBlur = async () => {
    if (localValue !== apiKey) {
        await commands.changePostProcessApiKeySetting(
            "gemini_transcription",
            localValue,
        );
    }
};

<Input onBlur={handleBlur} />
```

**The bug**: When you click a tab button, the input loses focus (`onBlur` fires), but then React **immediately unmounts the component** before the async save completes. The promise never finishes!

**First attempt**: Use a cleanup effect
```typescript
useEffect(() => {
    return () => {
        // This runs on unmount
        if (localValue !== apiKey) {
            commands.changePostProcessApiKeySetting(
                "gemini_transcription",
                localValue
            );
        }
    };
}, [localValue, apiKey]);
```

**Why it failed**: Cleanup functions in React can't `await`. The function runs, but React doesn't wait for the async operation to complete. The component dies, the promise dies with it.

**Second attempt**: Debounced auto-save
```typescript
useEffect(() => {
    const timeout = setTimeout(async () => {
        if (localValue !== lastSavedValue) {
            await commands.changePostProcessApiKeySetting(...);
        }
    }, 500);

    return () => clearTimeout(timeout);
}, [localValue]);
```

**Why it failed**: Still the same unmount race condition. The timeout clears, but any in-flight save is lost.

**Final solution**: Explicit save button
```typescript
const handleSave = async () => {
    setIsSaving(true);
    try {
        await commands.changePostProcessApiKeySetting(...);
        await refreshSettings();  // ← CRITICAL: Update the store!
        setJustSaved(true);
    } finally {
        setIsSaving(false);
    }
};

{hasUnsavedChanges && (
    <button onClick={handleSave}>
        <Save />
    </button>
)}
```

**The lesson**:
- **Don't fight React's lifecycle**. If unmounting is the problem, prevent unmounting (keep the component visible) or give explicit control to the user.
- **Refresh after mutation**: The backend saved the data, but the frontend store (`useSettings()`) still has old data. Always refresh!

### Bug #2: TypeScript Can't Find The New Commands

**What happened**: Build fails with `Property 'updateHistoryEntryText' does not exist on type...`

**The investigation**:
```bash
$ bun run tauri build
error TS2339: Property 'updateHistoryEntryText' does not exist
```

But we clearly added the command in Rust! What gives?

**The root cause**: The `bindings.ts` file wasn't regenerated. Here's how Tauri generates bindings:

```rust
// src-tauri/src/lib.rs
#[cfg(debug_assertions)]  // ← Only in DEBUG builds!
specta_builder
    .export(
        Typescript::default(),
        "../src/bindings.ts",  // ← Writes here
    )
    .expect("Failed to export bindings");
```

**The sequence**:
1. We added the command to `lib.rs` ✅
2. We ran `cargo build --release` ❌ (release builds skip binding generation!)
3. Frontend still has old `bindings.ts` ❌
4. TypeScript compilation fails ❌

**The fix**:
```bash
# Run a DEBUG build first (generates bindings)
cd src-tauri && cargo build

# THEN run the release build
cd .. && bun run tauri build
```

**The lesson**:
- **Read build conditions carefully**: `#[cfg(debug_assertions)]` means "only in debug mode"
- **Auto-generated files need regeneration**: When you add/change commands, rebuild in debug mode
- **Watch for stale artifacts**: Delete `target/` if things get weird

### Bug #3: The Missing `immer` Dependency

**What happened**:
```bash
Error: The following dependencies are imported but could not be resolved:
  immer (imported by ModelSelector.tsx)
Are they installed?
```

**The investigation**:
- The original codebase uses `immer` in `ModelSelector.tsx`
- We forked the repo and cloned it
- But we ran `bun install`, which should install dependencies... right?

**The root cause**: The original `package.json` had `immer` in `devDependencies` but it was removed at some point. Our working directory didn't have it, but the original repo's `node_modules` did (from an earlier version).

**The fix**:
```bash
bun install immer
```

**The lesson**:
- **Check `package.json` after forking**: Dependencies can drift
- **Don't assume `node_modules` is complete**: When cloning, always `npm install` / `bun install`
- **Missing dependencies are easy to fix**: Just install them

### Bug #4: The Cached Build Path Nightmare

**What happened**:
```
failed to read plugin permissions:
failed to read file '\\?\C:\Users\hninw\Handy\src-tauri\target\...'
The system cannot find the path specified.
```

But we moved the project to `C:\Users\hninw\OneDrive\デスクトップ\Experiments\Handy`!

**The root cause**: Cargo (Rust's build tool) caches build artifacts in `target/`. When we moved the project, the cache still pointed to the old path.

**The fix**:
```bash
cd src-tauri
cargo clean  # Delete ALL build artifacts (11.4GB!)
cd ..
bun run tauri build  # Fresh build
```

**The lesson**:
- **`cargo clean` is your friend**: When paths change, clean the cache
- **Build artifacts are HUGE**: Rust `target/` directories can be 10+ GB
- **Absolute paths in caches break on move**: This is a common problem in all compiled languages

### Bug #5: The Windows SmartScreen "Error" (Not Actually an Error)

**What happened**: After building, the final step shows:
```
Error: A public key has been found, but no private key.
Make sure to set `TAURI_SIGNING_PRIVATE_KEY` environment variable.
error: script "tauri" exited with code 1
```

**The confusion**: The error appears AFTER the installers are created:
```
Finished 2 bundles at:
  C:\...\Handy_0.7.0_x64_en-US.msi
  C:\...\Handy_0.7.0_x64-setup.exe

Error: A public key has been found...
```

**The root cause**: Tauri has TWO signing systems:
1. **Windows code signing**: Signs the `.exe` to avoid SmartScreen warnings (we disabled this)
2. **Tauri auto-updater signing**: Signs update manifests so users can auto-update

The error is about #2, but we don't use auto-updates, so we ignore it.

**The fix**: None needed! The installers work fine. We could disable the updater in `tauri.conf.json` to remove the warning:
```json
{
    "plugins": {
        "updater": {
            "active": false  // ← Add this
        }
    }
}
```

**The lesson**:
- **Read error messages carefully**: "After success" errors are often warnings
- **Distinguish critical vs. cosmetic failures**: Installers work = success, even with warnings
- **Documentation helps**: The error message is confusing without context

---

## Working With a Fork: Managing Upstream Changes

### The Git Workflow We Set Up

**Problem**: We want to:
1. Keep `main` synchronized with the original repo (for updates)
2. Keep our custom features in a separate branch
3. Be able to rebase our features on top of new upstream changes

**Solution**: The "upstream + feature branch" pattern

```bash
# One-time setup
git remote add upstream https://github.com/cjpais/Handy.git
git fetch upstream

# Create feature branch from current main
git checkout -b custom-features

# Reset main to match upstream
git checkout main
git reset --hard upstream/main
git push origin main --force
```

**The structure**:
```
origin/main          ← Our fork's main (synced with upstream)
    ↑
upstream/main        ← Original repo (CJ Pais)

origin/custom-features  ← Our custom work
    ↑
(based on upstream/main v0.7.0)
```

### How to Update When Upstream Changes

**Scenario**: CJ Pais releases v0.8.0 with bug fixes. We want them!

**The workflow**:
```bash
# 1. Update main to latest upstream
git checkout main
git fetch upstream
git merge upstream/main  # or: git reset --hard upstream/main
git push origin main

# 2. Rebase our features on top of the new main
git checkout custom-features
git rebase main

# 3. Resolve any conflicts (if they exist)
# ... (edit files, git add, git rebase --continue)

# 4. Force push (because we rewrote history)
git push origin custom-features --force
```

**What can go wrong**: Merge conflicts!

Example conflict we hit:
```tsx
<<<<<<< HEAD
import { MuteWhileRecording } from "../MuteWhileRecording";
=======
import { GeminiApiKey } from "./GeminiApiKey";
>>>>>>> feat: add Gemini support
```

**How to resolve**:
```tsx
// Keep BOTH imports!
import { MuteWhileRecording } from "../MuteWhileRecording";
import { GeminiApiKey } from "./GeminiApiKey";
```

Then:
```bash
git add src/components/settings/general/GeneralSettings.tsx
git rebase --continue
```

**The lesson**:
- **Upstream is truth**: Sync main with upstream regularly
- **Feature branches are flexible**: Rebase to stay current
- **Conflicts are normal**: Don't panic, read the diff carefully
- **Test after rebasing**: Make sure everything still works!

### When to Fork vs. Contribute Upstream

**You should fork when**:
- The changes are specific to your use case (Burmese → English translation)
- You're experimenting and don't know if it'll work
- You want to move fast without PR review delays

**You should contribute upstream when**:
- The feature benefits everyone (bug fixes, performance improvements)
- It aligns with the project's vision
- You're willing to maintain it and respond to feedback

For this project, our features are pretty specific to your use case, so forking was the right call. But if you find a bug in the core transcription engine, consider submitting a PR upstream!

---

## Lessons Learned: Wisdom From The Trenches

### 1. Type Safety Is a Superpower

**Before**:
```javascript
// JavaScript - pray it works
function updateHistory(id, text) {
    callBackend('update', id, text);  // Did I get the args right?
}
```

**After**:
```typescript
// TypeScript + tauri-specta - compiler guarantees correctness
async function updateHistory(id: number, text: string) {
    await commands.updateHistoryEntryText(id, text);
    // ↑ If the backend changes, this line errors at compile time!
}
```

**The lesson**: Spending 5 minutes adding types saves 5 hours debugging "undefined is not a function" at runtime.

### 2. Event-Driven Architecture Scales

**The pattern**:
```rust
// Backend: Do work, then emit event
history_manager.save(...)?;
app_handle.emit("history-updated", ())?;  // ← Broadcast change
```

```typescript
// Frontend: Listen and react
listen("history-updated", () => {
    loadHistoryEntries();  // ← Auto-refresh
});
```

**Why this works**:
- **Decoupling**: Backend doesn't know who's listening (could be 0, could be 10 components)
- **Consistency**: All listeners update together
- **Easy to extend**: Want to show a notification when history updates? Just add another listener!

**The lesson**: Events are like a radio broadcast. The backend announces "history changed!" and anyone interested tunes in. No tight coupling.

### 3. Manager Objects Beat Scattered Logic

**Bad**:
```rust
// Scattered across multiple files
fn save_audio(path: &str) { /* ... */ }
fn load_audio(path: &str) { /* ... */ }
fn get_db_connection() { /* ... */ }
fn save_to_db(data: Data) { /* ... */ }
```

**Good**:
```rust
// One manager owns the domain
struct HistoryManager {
    db_path: PathBuf,
    recordings_dir: PathBuf,
}

impl HistoryManager {
    fn save_transcription(&self, ...) { /* use self.db_path */ }
    fn get_entry(&self, id: i64) { /* use self.db_path */ }
    // All history operations live here!
}
```

**The lesson**: When you need to change something (e.g., "use PostgreSQL instead of SQLite"), you edit **one file** (HistoryManager), not 20 scattered functions.

### 4. Migrations Save Your Users' Data

**The setup**:
```rust
static MIGRATIONS: &[M] = &[
    M::up("CREATE TABLE transcription_history (...)"),     // v1
    M::up("ALTER TABLE ... ADD COLUMN post_processed_text"), // v2
    M::up("ALTER TABLE ... ADD COLUMN post_process_prompt"), // v3
];
```

**What happens**:
- User installs v0.6.0 (has v1 schema)
- User upgrades to v0.7.0 (needs v3 schema)
- On startup, Handy automatically runs migrations v2 and v3
- User's data is preserved! 🎉

**The lesson**: Never write code that assumes a fresh database. Users upgrade, and migrations let you evolve the schema without losing their data.

### 5. Error Messages Are For Humans

**Bad**:
```rust
.map_err(|e| e.to_string())  // "error code 5"
```

**Good**:
```rust
.map_err(|e| format!("Failed to save transcription to database: {}", e))
```

**Even better**:
```typescript
catch (error) {
    alert(
        error instanceof Error
            ? error.message
            : "Failed to save. Please check your Gemini API key in Settings."
    );
}
```

**The lesson**: When something breaks, tell the user **what went wrong** and **what to do about it**. "Error code 5" is useless. "Failed to save. Please check your Gemini API key" is actionable.

### 6. Build Systems Have Modes For a Reason

**Debug builds** (`cargo build`):
- Generate TypeScript bindings ✅
- Include debug symbols (for debugging)
- Slow, large binaries (~500MB)
- Fast compile times (~8 min)

**Release builds** (`cargo build --release`):
- Skip binding generation ❌
- Optimize for size/speed
- Small, fast binaries (~30MB)
- Slow compile times (~15 min)

**The lesson**: Run debug builds during development (fast iteration), release builds for distribution (small, fast app).

### 7. Dependencies Are a Double-Edged Sword

**Handy has 715 Rust dependencies**. That sounds like a lot! But:

**Pros**:
- Don't reinvent the wheel (audio I/O, SQLite, HTTP clients, etc.)
- Battle-tested code (millions of downloads)
- Security updates (just `cargo update`)

**Cons**:
- Supply chain risk (what if a dependency gets hacked?)
- Compilation time (15 min initial build)
- Version conflicts (dependency A needs B v1, dependency C needs B v2)

**The lesson**: Use dependencies for hard problems (audio processing, AI inference). Write your own code for simple problems (button click handlers). Audit your dependencies occasionally (`cargo tree`).

### 8. Code Signing Is a Pain (But Worth It)

**Unsigned apps**:
- Windows: "Unknown publisher" warning
- macOS: "Cannot be opened because developer cannot be verified" (user must right-click → Open)
- User trust: Low

**Signed apps**:
- Windows: No warning (if you pay $200/year for a certificate)
- macOS: No warning (if you pay $99/year + notarize)
- User trust: High

**The lesson**: For serious projects, budget for code signing certificates. For experiments, warn users about the SmartScreen pop-up.

---

## Best Practices: How Good Engineers Think

### 1. Read Before You Write

**The scenario**: You want to add a feature. Where do you start?

**Bad approach**:
```
1. Immediately open an editor
2. Start writing code
3. Realize you don't understand the existing architecture
4. Refactor everything
5. Break existing features
```

**Good approach**:
```
1. Read the existing code
2. Find similar features (how did they solve this?)
3. Understand the patterns (manager objects, commands, events)
4. Copy the pattern for your new feature
5. Test thoroughly
```

**We did this**: When adding "edit transcription", we first read `toggle_saved_status()`. It showed us the pattern:
1. Create a command in `commands/history.rs`
2. Implement logic in `managers/history.rs`
3. Emit "history-updated" event
4. Frontend auto-refreshes

Copy the pattern, change the details. Don't reinvent.

### 2. Make It Work, Then Make It Pretty

**First attempt** (working but ugly):
```typescript
const handleSave = async () => {
    await commands.updateHistoryEntryText(entry.id, editedText);
    setIsEditing(false);
};
```

**Second iteration** (working and pretty):
```typescript
const handleSave = async () => {
    setIsSaving(true);
    try {
        await commands.updateHistoryEntryText(entry.id, editedText);
        await refreshSettings();  // Refresh the store!
        setIsEditing(false);
        setJustSaved(true);
        setTimeout(() => setJustSaved(false), 2000);
    } catch (error) {
        console.error("Failed to update:", error);
        alert("Failed to save changes. Please try again.");
    } finally {
        setIsSaving(false);
    }
};
```

**The lesson**: Get it working first. Then add error handling, loading states, and polish. Don't try to write perfect code on the first attempt.

### 3. When in Doubt, Log It Out

**Debugging tools**:
```rust
// Backend (Rust)
use log::{debug, info, warn, error};

debug!("About to save transcription for entry {}", id);
info!("Translation succeeded: {} chars", translated.len());
warn!("Gemini API returned empty response");
error!("Failed to connect to database: {}", e);
```

```typescript
// Frontend (TypeScript)
console.log("Starting translation...");
console.error("Translation failed:", error);
```

**Where logs go**:
- Rust logs: `AppData/Roaming/com.pais.handy/logs/`
- Browser console: DevTools (Ctrl+Shift+I)

**The lesson**: Logs are breadcrumbs. When something breaks, logs tell you **what happened** and **when**.

### 4. Test The Happy Path, Then The Sad Path

**Happy path**: Everything works perfectly
```typescript
// User enters valid API key
// Clicks Save
// Switches tabs
// Returns → API key is still there ✅
```

**Sad paths**: Everything that can go wrong
```typescript
// User enters API key, but internet is down when they translate
// User switches tabs while saving
// User clicks Save twice rapidly
// User closes the app mid-save
// Backend returns an error
```

**The lesson**: Most bugs live in sad paths. Test error cases!

### 5. Commit Messages Tell a Story

**Bad**:
```bash
git commit -m "fix stuff"
git commit -m "more changes"
git commit -m "wip"
```

**Good**:
```bash
git commit -m "feat: add edit button for history transcriptions

- Add Pencil icon button to history entries
- Edit mode shows textarea with Save/Cancel buttons
- Backend command updates database and emits event
- Frontend refreshes automatically on save

Fixes issue where users couldn't correct Gemini's mistakes."
```

**The lesson**: Future you (or future maintainers) will read this commit. Explain **what** changed, **why** it changed, and **how** it works.

### 6. Naming Things Is Hard, But Important

**Bad names**:
```rust
fn do_thing(x: i64, y: String) -> Result<(), String> { ... }
```

**Good names**:
```rust
fn update_transcription_text(
    entry_id: i64,
    new_text: String
) -> Result<(), String> { ... }
```

**The lesson**: Code is read 10x more than it's written. Spend an extra 10 seconds choosing clear names. Your teammates (and future you) will thank you.

---

## What's Next: Future Considerations

### Performance Optimizations

**Current state**: The app works, but there's room for improvement:

**1. Database Indexing**
```sql
-- Current (no index on timestamp)
SELECT * FROM transcription_history
WHERE saved = 0
ORDER BY timestamp DESC;  -- ← Sequential scan!

-- Future (add index)
CREATE INDEX idx_timestamp ON transcription_history(timestamp);
CREATE INDEX idx_saved_timestamp ON transcription_history(saved, timestamp);
```

**2. Lazy Loading**
Currently, `getHistoryEntries()` loads ALL entries. What if you have 10,000?

```rust
// Future: Pagination
pub async fn get_history_entries(
    &self,
    offset: usize,
    limit: usize,
) -> Result<Vec<HistoryEntry>> {
    // Load 50 at a time, on-demand
}
```

**3. Translation Caching**
If you translate the same text twice, why call Gemini again?

```rust
// Cache structure: HashMap<original_text, translation>
let mut cache: HashMap<String, String> = HashMap::new();

if let Some(cached) = cache.get(&text) {
    return Ok(cached.clone());  // Instant!
}
```

### Feature Ideas

**1. Batch Translation**
"Translate all Burmese entries to English"
- UI: Checkbox select + "Translate Selected" button
- Backend: Process in parallel (10 at a time)
- Progress bar: "Translating 45 of 120..."

**2. Export History**
"Download my transcriptions as CSV"
```rust
pub async fn export_history_csv(&self) -> Result<PathBuf> {
    let path = self.app_data_dir.join("history_export.csv");
    // Write CSV...
    Ok(path)
}
```

**3. Search History**
"Find transcriptions containing 'meeting notes'"
```sql
SELECT * FROM transcription_history
WHERE transcription_text LIKE '%meeting notes%'
ORDER BY timestamp DESC;
```

### Code Quality Improvements

**1. Remove Unused Imports**
Those warnings we saw?
```rust
warning: unused import: `anyhow::Result`
warning: variable does not need to be mutable
```

Fix with: `cargo fix --lib -p handy`

**2. Add Unit Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_transcription_text() {
        let manager = HistoryManager::new_in_memory();
        let entry = manager.save_transcription("Hello", ...);
        manager.update_transcription_text(entry.id, "Hi").await.unwrap();

        let updated = manager.get_entry(entry.id).await.unwrap();
        assert_eq!(updated.transcription_text, "Hi");
    }
}
```

**3. Extract Magic Strings**
```rust
// Current
.get("gemini_transcription")

// Better
const GEMINI_API_KEY: &str = "gemini_transcription";
.get(GEMINI_API_KEY)
```

### Attribution and Licensing

**Current state**:
- Original author: CJ Pais
- License: MIT (very permissive)
- Fork: Your custom version

**Recommendations**:
1. **Keep the MIT license** (required by law)
2. **Add your attribution**:
   ```md
   ## Attribution

   This is a fork of [Handy by CJ Pais](https://github.com/cjpais/Handy).

   Custom features by [Your Name]:
   - Edit transcription text
   - Translate Burmese to English via Gemini
   ```

3. **Consider upstreaming**:
   - If your edit feature is useful to everyone, submit a PR to the original repo
   - Translation might be too specific, but editing is universal!

### Deployment and Distribution

**Current state**: Manual installs from GitHub releases

**Future improvements**:

**1. Auto-Updates**
Tauri has a built-in updater:
```json
// tauri.conf.json
{
    "plugins": {
        "updater": {
            "active": true,
            "endpoints": ["https://yourserver.com/updates.json"]
        }
    }
}
```

**2. Code Signing**
Get certificates:
- **Windows**: SignPath Foundation (free for OSS) or DigiCert ($200/year)
- **macOS**: Apple Developer Program ($99/year)

**3. Package Managers**
- **Windows**: Winget, Chocolatey
- **macOS**: Homebrew (`brew install handy`)
- **Linux**: apt, yum, snap

### Monitoring and Analytics

**What to track** (privacy-friendly):
- Crash reports (Sentry)
- Feature usage (e.g., "How many users use translation?")
- Performance metrics (transcription latency)

**What NOT to track**:
- Audio data (privacy nightmare!)
- Transcription content (user's personal data)
- Keystrokes (creepy!)

---

## Final Thoughts

Building this project taught us:
1. **Modern desktop apps are complex** - But frameworks like Tauri make it manageable
2. **Type safety saves time** - Rust + TypeScript catch bugs before they ship
3. **Event-driven architecture scales** - Decoupled systems are easy to extend
4. **Good code is obvious code** - Patterns, naming, structure matter more than cleverness
5. **Bugs are teachers** - Each bug reveals something about the system
6. **Open source is a superpower** - We built on thousands of hours of work by others

**Most importantly**: You don't need to understand everything before you start. We learned about:
- Tauri's command system by adding commands
- React state management by fixing the API key bug
- Rust async programming by implementing translation
- Git workflows by managing a fork

**The best way to learn is to build, break, fix, and iterate.**

Now go build something awesome! 🚀

---

*Written with the scars and wisdom from debugging production issues at 2 AM.*
