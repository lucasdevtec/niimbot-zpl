# ZPL suportado

Subconjunto mínimo de ZPL II, implementado em `src/zpl.rs`. Comandos desconhecidos são ignorados em silêncio.
Resolução: 203 dpi (≈ 8 pontos/mm). Apenas a primeira etiqueta do texto é renderizada.

| Comando | Efeito |
|---|---|
| `^XA` / `^XZ` | Delimitam a etiqueta (não são obrigatórios para renderizar) |
| `^PW w` | Largura em pontos, limitada a 8–384 (padrão 384) |
| `^LL h` | Altura em pontos, 8–4000 (padrão 240) |
| `^FO x,y` / `^FT x,y` | Posição do próximo campo (`^FT` é tratado como `^FO`) |
| `^CF f,h` | Altura de fonte padrão |
| `^A0N,h,w` | Altura da fonte do próximo texto (a largura é ignorada) |
| `^FD dados` | Dados: texto, ou conteúdo do QR se precedido por `^BQ` |
| `^FS` | Fim do campo; desenha a caixa pendente de `^GB` |
| `^GB w,h,t` | Caixa com borda de espessura `t`, desenhada no `^FS` |
| `^BQN,2,m` | QR Code com módulo `m` (1–10); os dados vêm em `^FDQA,conteúdo` |

## Texto

Fonte bitmap 8×8 (`font8x8`), ampliada por um fator inteiro `altura ÷ 8` (mínimo 1). Portanto:

- tamanhos múltiplos de 8 (16, 24, 32, 40…) ficam nítidos; outros são arredondados para baixo;
- largura do texto = `caracteres × 8 × fator`;
- não há quebra de linha automática nem negrito.

## Exemplo

```zpl
^XA
^PW384
^LL240
^FO20,20^A0N,40,40^FD{{nome}}^FS
^FO20,90^BQN,2,5^FDQA,{{nome}}^FS
^FO200,90^GB150,100,3^FS
^XZ
```

`{{nome}}` é substituído pelas [variáveis](uso.md#variáveis-e-impressão-em-lote) antes da renderização.

## Não suportado

Códigos de barras (`^BC` etc.), imagens (`^GF`), rotação, outras fontes, `^FB` (bloco de texto), `^FR`, `^PQ`.
