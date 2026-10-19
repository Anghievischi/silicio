//! Testes de fumaça do executável real.
//!
//! `abre_a_janela` abre a interface, passa por todas as abas e confere que o
//! processo termina sem erro. Roda sempre no Windows (o CI tem área de
//! trabalho) e no Linux só com `SILICIO_SMOKE=1`, para não abrir janelas
//! durante um `cargo test` comum.
//!
//! Precisa do front compilado (`pnpm build`) antes de compilar os testes.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_silicio");

#[test]
fn relatorio_json_e_valido() {
    let out = Command::new(BIN).arg("--json").output().expect("executa o silicio");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON válido");

    assert!(report["cpu"]["threads"].as_u64().unwrap_or(0) > 0);
    assert!(report["memory"]["total_bytes"].as_u64().unwrap_or(0) > 0);
    if cfg!(windows) {
        // No Windows estes dados vêm do WMI; vazios indicariam falha na consulta.
        assert!(report["board"]["bios"]["version"].is_string(), "BIOS: {}", report["board"]);
        assert!(report["gpu"].as_array().is_some_and(|g| !g.is_empty()), "GPU: {}", report["gpu"]);
    }
}

#[test]
fn abre_a_janela() {
    if !cfg!(windows) && std::env::var_os("SILICIO_SMOKE").is_none() {
        eprintln!("ignorado: defina SILICIO_SMOKE=1 para abrir a janela no Linux");
        return;
    }

    let mut child = Command::new(BIN)
        .arg("--smoke-test")
        .env("RUST_BACKTRACE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("executa o silicio");

    // O modo --smoke-test fecha sozinho depois de ~22 s.
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("consulta o processo") {
            break status;
        }
        if start.elapsed() > Duration::from_secs(90) {
            let _ = child.kill();
            panic!("o app não fechou sozinho em 90 s");
        }
        std::thread::sleep(Duration::from_millis(250));
    };

    let out = child.wait_with_output().expect("lê a saída");
    assert!(
        status.success(),
        "o app terminou com {status}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(start.elapsed() > Duration::from_secs(15), "o app fechou cedo demais");
}
