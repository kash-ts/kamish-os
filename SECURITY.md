# Security Policy: Kamish OS

Please refer to the full security specification, threat model, and AI agent guardrails in [docs/SECURITY.md](docs/SECURITY.md).

## Quick Summary of Invariants
- **Local Audio Only**: STT and TTS are strictly on-device (Vosk/Piper). No audio data leaves the speaker.
- **AI Sandboxing**: The LLM has zero shell, filesystem, or network socket access. Device control is strictly restricted to a typed 14-tool registry.
- **Hardware Protection**: Programmatic safe volume ceiling at 85% to protect physical audio components.
- **Reporting Vulnerabilities**: Send reports to `security@kamish-os.org` or open a private GitHub Security Advisory.
