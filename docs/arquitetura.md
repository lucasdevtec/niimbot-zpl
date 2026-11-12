# Arquitetura

```
 ┌──────────────┐  Cmd::Batch / Calibrate   ┌────────────────────┐   bytes   ┌──────────┐
 │  UI (Slint)  │ ────── mpsc channel ────▶ │ worker (thread)    │ ────────▶ │ Transport│─▶ B1
 │  main thread │ ◀── invoke_from_event_loop│ Printer<UsbSerial> │ ◀──────── │ (USB)    │
 └──────┬───────┘      status / progress    └────────────────────┘           └──────────┘
        │ edits
        ▼
  State { elementos, seleção, colunas, linhas }
        │ designer::to_zpl  →  data::substitute  →  zpl::render  →  GrayImage
        ▼
  preview (Image)  +  caixas de seleção (El)  ·  GrayImage → protocol::Printer::print
```

## Módulos (`src/`)

| Módulo | Responsabilidade |
|---|---|
| `transport.rs` | Trait `Transport` (`write_all`, `read`), implementação `UsbSerial` (115200, CDC), `find_niimbot()` e `list_ports()`. |
| `protocol.rs` | Codificação/decodificação de pacotes, `Printer<T: Transport>`: `print`, `battery`, `firmware`, `calibrate`, `heartbeat`. |
| `zpl.rs` | `render(&str) -> GrayImage`, mais `text_extent` e `qr_size` (métricas usadas pelo designer). |
| `designer.rs` | `Element` (Texto/QR/Caixa), `to_zpl`, `bbox`. |
| `data.rs` | `parse_csv` e `substitute` (`{{coluna}}`). |
| `main.rs` | Estado da UI, callbacks e a thread do worker. |
| `ui/app.slint` | Interface; compilada por `build.rs`. |

`lib.rs` expõe tudo menos a UI, então protocolo, ZPL e dados são testáveis sem janela.

## Threads

- **UI (principal):** edita o estado, gera as imagens do lote e envia `Cmd` pelo canal.
- **Worker:** dono da conexão. A cada 2 s (ou ao receber um comando) detecta a porta se não houver conexão,
  executa o comando e consulta bateria/firmware. Se a consulta falha, descarta a conexão e tenta de novo na rodada
  seguinte. Atualiza a UI com `upgrade_in_event_loop`.

## Fluxo de preview

`refresh()` em `main.rs` recalcula tudo a cada alteração: gera o ZPL (do designer ou do editor), substitui as
variáveis da linha em pré-visualização, renderiza o bitmap, calcula as caixas de seleção por elemento e preenche o
painel de propriedades. O modelo das caixas é atualizado **no lugar** (`set_row_data`); trocar o modelo cancelaria
um arrasto em andamento.

## Arrastar no canvas

Cada elemento tem uma área de toque sobreposta à imagem. O evento `moved` envia o deslocamento em pontos
(pixels ÷ escala) e o Rust soma à posição, limitando ao tamanho da etiqueta. A escala do canvas é calculada para a
etiqueta caber na área disponível (máx. 4×).
