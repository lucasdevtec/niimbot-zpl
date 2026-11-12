# CI e releases

O projeto usa dois workflows do GitHub Actions em `.github/workflows/`.

## CI (`ci.yml`)

Roda a cada push na `main` e em todo pull request, em **Linux** (`ubuntu-latest`) e **Windows** (`windows-latest`).

| Passo | Comando |
|---|---|
| Formatação | `cargo fmt --check` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Testes | `cargo test` |
| Build | `cargo build --release` |

Qualquer aviso do clippy falha o CI. Para reproduzir localmente:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

No Linux o workflow instala `libfontconfig1-dev`, `libxkbcommon-dev` e `libwayland-dev` (necessários ao Slint).
O `libudev` não é necessário porque o `serialport` está com as features padrão desligadas.

## Release (`release.yml`)

Dispara ao enviar uma tag `v*` ou manualmente (*Actions → Release → Run workflow*).

Gera um pacote por plataforma, sempre com `README.md`, `LICENSE` e `exemplo.csv`:

| Plataforma | Arquivo | Extra |
|---|---|---|
| Linux x86_64 | `niimbot-zpl-x86_64-unknown-linux-gnu.tar.gz` | `99-niimbot.rules` (udev) |
| Windows x86_64 | `niimbot-zpl-x86_64-pc-windows-msvc.zip` | |

- Em **tag**, os pacotes são anexados a uma GitHub Release com notas geradas automaticamente.
- Em execução **manual**, os pacotes ficam só como artefatos do workflow.
- O build usa `--locked`: o `Cargo.lock` precisa estar commitado e atualizado.

### Publicar uma versão

1. Atualize `version` no `Cargo.toml` e faça commit.
2. Crie e envie a tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. Aguarde o workflow e confira a página *Releases* do repositório.

## Observações por plataforma

- **Windows:** o `.exe` de release abre sem janela de console (`windows_subsystem = "windows"`), então os logs de
  protocolo em `stderr` só aparecem em builds de debug (`cargo run`). A B1 precisa aparecer como porta COM; se o
  Windows não instalar o driver sozinho, a detecção automática não encontra a impressora.
- **Linux:** para acessar `/dev/ttyACM*` sem ser do grupo `dialout`, instale a regra udev incluída no pacote
  (instruções no [README](../README.md#permissão-da-porta-usb-linux)).

## Limitações conhecidas

- Apenas x86_64. macOS e ARM não estão no build.
- Os binários não são assinados; o Windows SmartScreen pode exibir um aviso na primeira execução.
- Os workflows ainda não foram executados no GitHub: o primeiro run pode exigir ajustes.
