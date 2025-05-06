# 🧊 Iced Hot Reload Example

This project demonstrates a dynamically reloaded GUI application using [Iced](https://github.com/iced-rs/iced) in Rust. The UI logic is compiled separately as a dynamic library (`cdylib`), which can be hot-swapped at runtime without restarting the application.

---

## 📂 Project Structure

```text
.
├── app_core      # UI + logic compiled as a shared library
├── app_shell     # Host binary loading and running core
├── shared_types  # Traits, messages, and shared state across crates
```

---

## 🚀 Features

* Hot-reloads core application logic using `libloading`
* Preserves app state (`AppState`) across reloads

---

## ⚙️ Building & Running

1. Clone the repo and navigate to the workspace root
2. Build all crates:

   ```bash
   cargo build
   ```
3. Run the shell app:

   ```bash
   cargo run -p app_shell
   ```

Any changes to the UI or logic in `app_core` will trigger a reload after recompilation:
   ```bash
   cargo build -p app_core
   ```

---

## 🌍 Platform Support

* Windows: `.dll`

File extension is resolved dynamically with `cfg!` at runtime.

---

## ✉️ License

MIT or Apache-2.0
