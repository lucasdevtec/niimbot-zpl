# Protocolo da Niimbot B1

Baseado em engenharia reversa da comunidade ([niimprint](https://github.com/kjy00302/niimprint),
[niimblue](https://github.com/MultiMote/niimblue)). Implementado em `src/protocol.rs`. Validado na prática com a
B1 via USB: impressão, bateria/firmware e calibração. Os campos ainda não confirmados estão em
[Incertezas](#incertezas).

## Pacote

```
55 55 | cmd | len | data[len] | checksum | AA AA
```

`checksum = cmd ^ len ^ data[0] ^ … ^ data[len-1]`. `decode` descarta pacotes com checksum inválido e
ressincroniza procurando `55 55`.

## Comandos usados

| Cmd | Nome | Dados |
|---|---|---|
| `0x21` | Densidade | `[1..5]` |
| `0x23` | Tipo de etiqueta | `[1]` = com vão (gap) |
| `0x01` | Início do job | `[1]` |
| `0x03` | Início da página | `[1]` |
| `0x13` | Tamanho da página | `altura:u16 BE, largura:u16 BE, cópias:u16 BE` |
| `0x84` | Linha vazia | `linha:u16 BE, repetições:u8` |
| `0x85` | Linha de bitmap | `linha:u16 BE, 0, pretos:u8, 0, 1, bytes…` |
| `0xE3` | Fim da página | `[1]` |
| `0xA3` | Status da impressão | `[1]` |
| `0xF3` | Fim do job | `[1]` |
| `0x40` | Info | `[chave]`: 8 modelo, 9 firmware, 10 bateria, 11 serial |
| `0x8E` | Calibrar | `[1]` (avança ~4 etiquetas) |
| `0xDC` | Heartbeat | `[1]` |

## Sequência de impressão

1. Densidade → tipo de etiqueta → início do job → início da página → tamanho da página.
2. Uma mensagem por linha da imagem (`0x84` se a linha for branca, `0x85` caso contrário). Não esperam resposta.
3. Fim da página.
4. Consulta `0xA3` a cada 300 ms até a página impressa ≥ cópias (timeout `30 s + 5 s × cópias`).
5. Fim do job.

O id da resposta é normalmente `cmd + 1`; as exceções são densidade (`0x21` → `0x31`), tipo de etiqueta (`0x23` → `0x33`)
e status (`0xA3` → `0xB3`). Pacotes com outro id (notificações espontâneas) são ignorados; se só chegarem pacotes
inesperados até o timeout, o último é usado e o fato é registrado no log (`!!`).

Os comandos de configuração, início e fim de página exigem resposta com primeiro byte `1`; caso contrário o app
reporta `printer rejected cmd 0x..`. Antes de cada requisição o buffer de leitura é esvaziado, para que um pacote
atrasado não seja tomado como resposta. Todo tráfego é registrado em `stderr` (`->` enviado, `<-` recebido).

## Imagem

- Cabeça de impressão: **384 pontos** (48 bytes) a 203 dpi. Imagens mais largas são rejeitadas.
- Cada linha da imagem é uma linha ao longo do avanço do papel; 1 bit por ponto, MSB primeiro, preto = pixel < 128.
- A imagem **não é centralizada**: o bit 0 é o ponto mais à esquerda. O campo de pretos é a contagem total
  saturada em 255.

## Incertezas

- A interpretação dos bytes de resposta de `0xA3` (página em `data[1]`) e do campo de contagem do `0x85` veio de
  implementações de terceiros e pode variar entre modelos/firmwares.
- `0x8E` foi observado fazendo a impressora avançar ~4 etiquetas; a documentação de terceiros o descreve como
  calibração, mas há fontes citando também `0x59`.
- O heartbeat (`0xDC`) é enviado mas a resposta não é interpretada, então tampa aberta e falta de papel ainda não
  são exibidos.
