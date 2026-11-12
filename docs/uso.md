# Guia de uso

## Conexão

Ligue a B1 por USB e abra o app. A impressora é detectada automaticamente (vendor id `0x3513`, ou qualquer
`/dev/ttyACM*` como alternativa) e reconectada se o cabo for removido. O estado aparece no topo:

- **B1 conectada · fw X.XX** e **Bateria N%**, atualizados a cada 2 segundos.
- **Impressora não encontrada** / **Conexão perdida**: confira o cabo e as permissões
  (veja [README](../README.md#permissão-da-porta-usb-linux)).

Os botões de imprimir e calibrar só ficam ativos com a impressora conectada.

## Modo Componentes

Barra superior: **Componentes** | **ZPL**.

- **Adicionar:** Texto, QR Code ou Caixa. O item novo fica selecionado.
- **Mover:** arraste o item no canvas. Clique no fundo para desmarcar.
- **Propriedades:**
  - Texto: conteúdo e tamanho (altura em pontos; múltiplos de 8 dão a melhor aparência).
  - QR Code: conteúdo e tamanho do módulo.
  - Caixa: largura, altura e espessura da borda.
- **Etiqueta:** largura (máx. 48 mm) e altura em mm. 1 mm ≈ 8 pontos (203 dpi). Padrão 48 × 30 mm = 384 × 240.
- **Remover item** apaga o item selecionado.

O canvas mostra o mesmo bitmap que será impresso.

## Modo ZPL

Edite o código ZPL diretamente. Ao trocar de Componentes para ZPL, o desenho atual é convertido em código
e colocado no editor. A volta não existe: o que você escreve em ZPL não vira componentes. Comandos suportados
em [zpl.md](zpl.md).

## Variáveis e impressão em lote

1. Use `{{coluna}}` em qualquer texto, no conteúdo do QR ou no código ZPL.
2. **Importar CSV…** e escolha o arquivo. A primeira linha define os nomes das colunas. Delimitador `;`, tab ou
   vírgula (detectado pela primeira linha); campos entre aspas são aceitos.
3. Cada coluna vira um botão: clique para inserir `{{coluna}}` no item selecionado (ou ao final do ZPL).
4. **Pré-visualizar** escolhe qual linha aparece no canvas.
5. O botão passa a ser **Revisar e imprimir N etiquetas**: uma etiqueta por linha. O campo **Cópias** vale
   para cada etiqueta.

```csv
nome;setor
Ana Souza;TI
Bruno Lima;RH
```

Detalhes:
- Variáveis sem coluna correspondente permanecem literais (`{{x}}`).
- `^` e `~` são removidos dos valores importados, para não quebrar o ZPL.
- Linhas em branco são ignoradas. **Limpar** descarta os dados importados.
- Cada etiqueta do lote é um job de impressão separado. Se uma falhar, o lote para e a mensagem indica qual.

## Revisar antes de imprimir

O botão **Revisar e imprimir** abre uma janela com a etiqueta final (o mesmo bitmap que vai para a impressora),
o tamanho em mm, a densidade e as cópias. Em lote, **◀ ▶** navegam entre as etiquetas e o botão confirma com
**Imprimir N**. **Cancelar** (ou clicar fora) fecha sem imprimir. Se a impressora estiver desconectada a prévia
abre, mas a impressão fica desabilitada.

## Impressão

- **Densidade** 1–5. Padrão 5 (mais escuro, máximo da B1).
- **Cópias** 1–99.
- Imprima uma etiqueta de teste antes de um lote grande.

## Calibrar papel

**Calibrar papel** envia o comando de calibração: a impressora avança cerca de **4 etiquetas** para medir o vão.
Use ao trocar o rolo ou se a impressão sair desalinhada. É sempre manual.
