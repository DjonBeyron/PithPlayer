# Pith Player v5.0.0

Плеер на Rust: движок **libmpv**, интерфейс **egui**.
Перенос с версии 4 (C# / WinForms / LibVLC).

- План работ: [PLAN.md](PLAN.md)
- Правила разработки: [CLAUDE.md](CLAUDE.md)

## Требования к среде

| Компонент | Состояние | Примечание |
|---|---|---|
| Rust 1.97+ | установлен | `C:\PITH\rust` |
| MinGW-w64 | установлен | `C:\PITH\tools\mingw64` |
| 7-Zip | установлен | нужен для распаковки SDK |
| FFmpeg | установлен | в `PATH`, понадобится с этапа 4 |
| **libmpv SDK** | **нужно поставить** | см. ниже |

Собираем GNU-цепочкой (`rust-toolchain.toml`): Visual Studio установлена без
компонента C++, линкера MSVC нет. GNU-вариант удобнее ещё и тем, что libmpv
поставляется с `libmpv.dll.a`, который линкуется напрямую.

## Установка libmpv

1. Скачать `mpv-dev-x86_64-*.7z` со страницы релизов
   [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)
   (около 40 МБ).
2. Распаковать так, чтобы получилось:

```
third_party/mpv/
├── include/mpv/       # client.h, render.h, render_gl.h
├── lib/libmpv.dll.a   # для линковки
└── libmpv-2.dll       # рантайм
```

3. Скопировать `libmpv-2.dll` рядом с собранным `pith-player.exe`
   (или в `target/debug/`).

Папка `third_party/mpv/` не хранится в репозитории — она в `.gitignore`.

## Сборка и запуск

```bash
cargo build
```

```bash
cargo run -- "C:\путь\к\видео.mkv"
```

Проверки перед коммитом (обязательны, см. CLAUDE.md):

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Контроль размера файлов — не более 400 строк:

```bash
find crates -name "*.rs" -exec wc -l {} + | awk '$1 > 400 {print "ПРЕВЫШЕНИЕ:", $2, $1}'
```

## Поставка

Портативный ZIP: распаковать и запустить, установка не нужна.

```bash
powershell -File scripts\package.ps1
```

Ключи: `-WithFfmpeg` вкладывает `ffmpeg.exe` и `ffprobe.exe` из `PATH`
(без них плеер работает, недоступна только нарезка), `-Portable` кладёт
`portable.txt` и переносит данные пользователя в саму папку, `-SkipBuild`
пропускает пересборку. Результат — в `dist/`.

## Уровень логирования

```bash
set PITH_LOG=debug && cargo run
```

## Состав

| Крейт | Назначение |
|---|---|
| `pith-mpv` | обёртка над libmpv: движок, контекст отрисовки, события, свойства |
| `pith-app` | приложение: окно, интерфейс egui, замеры |

Остальные крейты (`pith-subs`, `pith-fragments`, `pith-store`) добавляются
на своих этапах — пустых заготовок не держим.
