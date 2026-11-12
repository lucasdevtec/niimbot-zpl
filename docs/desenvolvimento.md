# Desenvolvimento

## Requisitos

- Rust estável.
- Linux: `libfontconfig1-dev`, `libxkbcommon-dev`, `libwayland-dev` (Debian/Ubuntu). Windows: nada extra.

## Comandos

```bash
cargo run --release     # abre o app
cargo test              # protocolo, ZPL, dados e designer (sem hardware)
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

`cargo run` (debug) mostra no terminal todo o tráfego com a impressora (`->` / `<-`). Use isso ao investigar
problemas de impressão e anexe o log ao reportar um bug. O mesmo log é gravado em `niimbot-zpl.log` na pasta
temporária do sistema (`/tmp/niimbot-zpl.log` no Linux; `%TEMP%` no Windows), útil em builds de release sem console.

## Testes

Os testes ficam junto dos módulos e não precisam de impressora: ida e volta de pacotes, renderização do ZPL,
CSV/substituição e geração de ZPL pelo designer. O que depende do hardware (sequência de impressão, calibração,
status) só é validado manualmente.

## Adicionar Bluetooth (BLE)

1. Implemente `Transport` (`write_all`, `read(timeout)`) para um tipo que use, por exemplo, `btleplug`
   (a B1 expõe uma característica de escrita/notificação).
2. Em `main.rs`, o worker hoje usa `Printer<UsbSerial>` e `transport::find_niimbot()`. Generalize para uma
   enumeração de conexões (USB/BLE) ou `Box<dyn Transport>` e adicione a busca por dispositivos BLE.
3. Pacotes BLE têm MTU limitado: pode ser necessário fragmentar `write_all`.

## Suportar outro modelo

As diferenças ficam em `protocol.rs`: largura da cabeça (`B1_WIDTH_PX`), tipo de etiqueta, formato do `0x13` e
das linhas `0x85`. Capture o tráfego do app oficial (Wireshark/`usbmon`) e compare com a sequência em
[protocolo.md](protocolo.md).

## Convenções

- Commits no estilo Conventional Commits (`feat(zpl): …`, `fix(protocol): …`, `docs: …`), um assunto por commit.
- CI exige `fmt`, `clippy -D warnings` e testes verdes.
