# WiseL-Project-Dev

[English](#english) | [Русский](#русский)

---

<a name="english"></a>
## 🇬🇧 English

### Requirements & Build
* **Rust** (stable toolchain)
* **FASM** (included in the `kernel/FASM/` directory)

To compile the compiler itself:
```bash
cargo build --release
```

To build a project using WiseL:
```
wisel build main.wise
```

### Overview
**WiseL-Project-Dev** is a lightweight, experimental programming language compiler designed for low-level systems development. It translates custom source files into optimized FASM assembly and compiles them into native executables for Windows (PE64) and Linux (ELF64).

### Key Features
* **Cross-Platform:** x64 Windows and Linux support. OS is auto-detected or can be set via `--target`.
* **Simple Syntax:** C/Rust-like, without unnecessary brackets or complex boilerplate.
* **Clear Output:** Generates a readable `.asm` file and a ready-to-run executable (`.exe` or ELF).

---

<a name="русский"></a>
## 🇷🇺 Русский

### Требования и сборка
* **Rust** (стабильная версия)
* **FASM** (уже включен в папку `kernel/FASM/`)

Чтобы собрать сам компилятор:
```bash
cargo build --release
```

Чтобы собрать проект на WiseL:
```bash
wisel build main.wise
```

### Обзор
**WiseL-Project-Dev** — это легковесный экспериментальный компилятор языка программирования, созданный для низкоуровневой системной разработки. Он преобразует исходный код в оптимизированный ассемблер FASM и собирает его в нативные исполняемые файлы для Windows (PE64) и Linux (ELF64).

### Основные особенности
* **Кроссплатформенность:** Поддержка x64 Windows и Linux. ОС определяется автоматически или задается через `--target`.
* **Простой синтаксис:** Похож на C/Rust, без лишних скобок и сложных конструкций.
* **Понятный результат:** На выходе вы получаете готовый `.asm` файл и скомпилированный исполняемый файл (`.exe` или ELF).
