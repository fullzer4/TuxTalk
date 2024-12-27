# 🐧 TuxTalk

TuxTalk is a voice-command assistant for Linux that lets you control your system using customizable voice commands. 🎙️ Built with Rust and powered by OpenAI's Whisper model, it offers seamless integration with your favorite tools and terminal commands.

## ✨ Features
- **Voice Recognition**: Uses OpenAI's Whisper model for accurate speech-to-text processing.
- **Custom Commands**: Define your own voice commands to perform any action on your system.
- **Shell Support**: Works with your preferred shell (e.g., `zsh`, `bash`).
- **Daemon Mode**: Runs in the background, always ready for your voice.

## 📦 Configuration
Easily configure TuxTalk using a simple INI-style configuration file:

```ini
[default]
prefix = "Tux"

[daemon]
audio_capture_enabled = true
audio_device = "default"

[whisper]
model = "base.en"

[shell]
type = "zsh"

[actions]
"open" = "i3-msg exec"
"close" = "pkill"

[commands]
"[open] browser" = "google-chrome-stable"
"[close] browser" = "google-chrome-stable"
"[open] terminal" = "kitty"
"[close] terminal" = "kitty"
```
