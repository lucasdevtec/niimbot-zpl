# niimbot-zpl

Cliente open source para a impressora térmica **Niimbot B1**, feito em Rust com [Slint](https://slint.dev).
Substitui o app oficial: você desenha a etiqueta (ou escreve em **ZPL**), importa uma planilha e imprime
uma etiqueta por linha, tudo direto pelo cabo USB.

> **Status:** MVP. Testado com a B1 via USB no Linux. Bluetooth ainda não implementado.

## Recursos

- **Conexão USB automática**: detecta a impressora (VID `0x3513`), reconecta se o cabo sair.
- **Status da B1**: bateria e versão do firmware.
- **Dois modos de edição**
  - *Componentes*: adicione Texto, QR Code e Caixa e arraste no canvas.
  - *ZPL*: edite o código direto. O desenho vira ZPL por baixo dos panos.
- **Variáveis e lote**: use `{{nome}}` nos textos/QR, importe um CSV e imprima uma etiqueta por linha.
- **Densidade máxima por padrão** (5) para marcação mais escura, número de cópias e calibração de papel.

## Instalação

Requer [Rust](https://rustup.rs) estável e, no Linux, as bibliotecas de desenvolvimento do `libudev`
(`libudev-dev` no Debian/Ubuntu, `systemd-devel` no Fedora).

```bash
git clone https://github.com/<seu-usuario>/niimbot-zpl
cd niimbot-zpl
cargo run --release
```

### Permissão da porta USB (Linux)

A B1 aparece como `/dev/ttyACM*`. Use uma das opções:

```bash
sudo usermod -aG dialout $USER            # e faça logout/login
# ou, sem grupo:
sudo cp contrib/99-niimbot.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

## Variáveis (impressão em lote)

1. Escreva `{{coluna}}` em um texto, QR ou no ZPL.
2. Clique em **Importar CSV…**. A primeira linha são os nomes das colunas; `;`, `,` e tab são aceitos.
3. O botão passa a dizer *Imprimir N etiquetas*. Use *Pré-visualizar* para conferir cada linha.

```csv
nome;setor
Ana Souza;TI
Bruno Lima;RH
```

Os caracteres `^` e `~` são removidos dos valores para não quebrar o ZPL.

## ZPL suportado

Subconjunto mínimo, o suficiente para texto, QR e caixas. Comandos desconhecidos são ignorados.

| Comando | Uso |
|---|---|
| `^XA` / `^XZ` | início/fim da etiqueta |
| `^PW` / `^LL` | largura (máx. 384) e altura em pontos |
| `^FO` / `^FT` | posição |
| `^A0N,h,w` / `^CF` | altura da fonte |
| `^FD` / `^FS` | dados do campo |
| `^GB w,h,t` | caixa |
| `^BQN,2,m` + `^FDQA,dados` | QR Code |

A resolução é 203 dpi (≈ 8 pontos/mm) e a cabeça da B1 tem 384 pontos (48 mm). O texto usa uma fonte
bitmap 8×8 ampliada. Códigos de barras (`^BC`) e imagens (`^GF`) ainda não são suportados.

## Arquitetura

```
src/
  transport.rs  trait Transport + USB serial (Bluetooth entra aqui)
  protocol.rs   pacotes Niimbot (55 55 … AA AA) e sequência de impressão
  zpl.rs        ZPL -> bitmap
  designer.rs   componentes arrastáveis -> ZPL
  data.rs       CSV e substituição de {{variáveis}}
  main.rs       UI Slint + thread que gerencia a impressora
ui/app.slint    interface
```

## Documentação

- [Índice da documentação](docs/README.md): uso, arquitetura, protocolo, ZPL, desenvolvimento, CI

## Roadmap

- [ ] Bluetooth (BLE) como segundo `Transport`
- [ ] Código de barras (`^BC`) e imagens (`^GF`)
- [ ] Salvar/abrir projetos de etiqueta
- [ ] Outros modelos (B21, D11, D110…)
- [ ] Ler tampa aberta / sem papel

## Aviso

Projeto independente, não afiliado à Niimbot. O protocolo foi baseado em engenharia reversa da comunidade
([niimprint](https://github.com/kjy00302/niimprint), [niimblue](https://github.com/MultiMote/niimblue)).
Use por sua conta e risco; o comando de calibração gasta algumas etiquetas.

## Licença

[MIT](LICENSE)
