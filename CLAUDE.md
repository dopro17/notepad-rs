# notepad-rs
Clone do Bloco de Notas em Rust (egui/eframe + muda).

## Comandos
- build: cargo build
- run: cargo run
- lint: cargo clippy -- -D warnings
- fmt: cargo fmt
- test: cargo test

## Regras
- Sem unwrap() em código de produção; usar anyhow/Result.
- Separar src/ em: app.rs (UI), document.rs (buffer, encoding, EOL), search.rs, settings.rs, platform/.
- Rodar fmt + clippy antes de concluir cada tarefa.

## Escopo (paridade com o Notepad)
Menus Arquivo/Editar/Formatar/Exibir/Ajuda, Novo/Abrir/Salvar/Salvar como,
desfazer/refazer, localizar/substituir, ir para linha, quebra de linha,
fonte, zoom, barra de status (Ln/Col, encoding, CRLF/LF), data/hora, arrastar arquivo.

## Plataforma
- Dev principal: macOS (Apple Silicon). Manter compatibilidade com Windows e Linux.
- Código específico de SO só em src/platform/{macos,windows,linux}.rs com #[cfg(target_os = "...")]; o resto agnóstico.
- Atalhos: usar `modifiers.command` do egui (⌘ no macOS, Ctrl nos demais). Nunca hardcodar Ctrl.
- Menus: `muda` (barra global no macOS; menu na janela em Windows/Linux). Mapear ⌘, (Preferências), ⌘Q, ⌘W, ⌘N, ⌘O, ⌘S, ⇧⌘S, ⌘F, ⌥⌘F (substituir), ⌘G, ⌘Z/⇧⌘Z, ⌘+/⌘-/⌘0.
- Menu "notepad-rs" (Sobre, Preferências, Ocultar, Sair) só em #[cfg(target_os = "macos")].
- Fonte monoespaçada padrão: Menlo/SF Mono (macOS), Consolas (Windows), DejaVu Sans Mono (Linux), com fallback embutido.
- EOL/encoding: detectar e preservar; padrão novo = UTF-8 + LF no macOS/Linux, CRLF no Windows.
- Empacotamento macOS: cargo bundle --release (gera .app).
