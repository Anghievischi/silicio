mod bench;
mod hw;

use bench::{BenchResult, Stress, StressStatus};
use hw::battery::{self, Battery};
use hw::board::{self, BoardInfo};
use hw::cpu::{self, CpuInfo, CpuLive, LiveSampler};
use hw::gpu::{Gpu, GpuLive, GpuSampler};
use hw::memory::{self, MemoryInfo, MemoryLive};
use hw::peripherals::{self, Peripherals};
use hw::privileged::PrivilegedInfo;
use hw::spd::{self, SpdModule};
use hw::storage::{self, Disk};
use hw::upgrade::{self, Upgrades};
use serde::Serialize;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::System;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

/// Executa a leitura num thread de trabalho.
///
/// Comandos síncronos do Tauri rodam no thread principal. No Windows várias
/// leituras usam WMI, que inicializa o COM em modo multithread; no thread da
/// janela (que usa COM single-thread) isso falha ou derruba o app.
async fn off_main<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

/// Trava um estado gerenciado, recuperando-o se outro thread entrou em pânico.
fn locked<T: Send + 'static, R>(app: &AppHandle, f: impl FnOnce(&mut T) -> R) -> R {
    let state = app.state::<Mutex<T>>();
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

#[tauri::command]
async fn cpu_info() -> Result<CpuInfo, String> {
    off_main(cpu::read_static).await
}

#[tauri::command]
async fn cpu_live(app: AppHandle) -> Result<CpuLive, String> {
    off_main(move || locked(&app, LiveSampler::sample)).await
}

#[tauri::command]
async fn board_info() -> Result<BoardInfo, String> {
    off_main(board::read).await
}

#[tauri::command]
async fn memory_info() -> Result<MemoryInfo, String> {
    off_main(memory::read).await
}

#[tauri::command]
async fn memory_live(app: AppHandle) -> Result<MemoryLive, String> {
    off_main(move || locked(&app, memory::read_live)).await
}

#[tauri::command]
async fn gpu_info(app: AppHandle) -> Result<Vec<Gpu>, String> {
    off_main(move || locked(&app, |s: &mut GpuSampler| s.info())).await
}

#[tauri::command]
async fn gpu_live(app: AppHandle) -> Result<Vec<GpuLive>, String> {
    off_main(move || locked(&app, |s: &mut GpuSampler| s.live())).await
}

/// Sugestões de upgrade (placa de vídeo, memória, processador).
#[tauri::command]
async fn upgrade_info(app: AppHandle) -> Result<Upgrades, String> {
    off_main(move || {
        let gpus = locked(&app, |s: &mut GpuSampler| s.info());
        upgrade::read(&gpus)
    })
    .await
}

#[tauri::command]
async fn peripherals_info() -> Result<Peripherals, String> {
    off_main(peripherals::read).await
}

#[tauri::command]
async fn storage_info() -> Result<Vec<Disk>, String> {
    off_main(storage::read).await
}

#[tauri::command]
async fn battery_info() -> Result<Vec<Battery>, String> {
    off_main(battery::read).await
}

#[tauri::command]
async fn spd_info() -> Result<Vec<SpdModule>, String> {
    off_main(spd::read).await
}

/// Roda o benchmark fora da thread principal; `threads` vazio usa todas as threads.
#[tauri::command]
async fn bench_run(threads: Option<usize>, seconds: u64) -> Result<BenchResult, String> {
    let threads = threads.unwrap_or_else(bench::available_threads);
    let seconds = seconds.clamp(1, 60);
    off_main(move || bench::benchmark(threads, seconds)).await
}

#[tauri::command]
fn stress_start(stress: tauri::State<'_, Mutex<Stress>>) -> StressStatus {
    let mut stress = stress.lock().unwrap_or_else(|e| e.into_inner());
    stress.start();
    stress.status()
}

#[tauri::command]
fn stress_stop(stress: tauri::State<'_, Mutex<Stress>>) -> StressStatus {
    let mut stress = stress.lock().unwrap_or_else(|e| e.into_inner());
    stress.stop();
    stress.status()
}

#[tauri::command]
fn stress_status(stress: tauri::State<'_, Mutex<Stress>>) -> StressStatus {
    stress.lock().unwrap_or_else(|e| e.into_inner()).status()
}

#[derive(Serialize)]
struct SystemInfo {
    app_version: &'static str,
    os: Option<String>,
    kernel: Option<String>,
    arch: &'static str,
}

/// Sistema operacional e versão do app, para o rodapé e o relatório.
#[tauri::command]
fn system_info() -> SystemInfo {
    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        os: System::long_os_version(),
        kernel: System::kernel_version(),
        arch: std::env::consts::ARCH,
    }
}

/// Prefixo obrigatório dos relatórios: o app só abre arquivos que ele mesmo salvou.
const REPORT_PREFIX: &str = "Silicio-";
const REPORT_EXTENSIONS: [&str; 4] = ["html", "md", "json", "png"];

fn report_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let paths = app.path();
    paths
        .download_dir()
        .or_else(|_| paths.document_dir())
        .or_else(|_| paths.home_dir())
        .map_err(|e| e.to_string())
}

fn is_report_name(name: &str) -> bool {
    let ext_ok = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| REPORT_EXTENSIONS.contains(&e));
    name.starts_with(REPORT_PREFIX)
        && ext_ok
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Grava na pasta Downloads, sem sobrescrever, e devolve o caminho.
fn write_report(app: &AppHandle, name: &str, contents: &[u8]) -> Result<String, String> {
    if !is_report_name(name) {
        return Err(format!("nome de arquivo inválido: {name}"));
    }
    let dir = report_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    let mut path = dir.join(name);
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn save_report(app: AppHandle, name: String, contents: String) -> Result<String, String> {
    write_report(&app, &name, contents.as_bytes())
}

/// Recebe a imagem como corpo binário da chamada e o nome no cabeçalho `name`.
#[tauri::command]
fn save_image(app: AppHandle, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("a imagem deve chegar como bytes".into());
    };
    let name = request
        .headers()
        .get("name")
        .and_then(|v| v.to_str().ok())
        .ok_or("cabeçalho name ausente")?;
    if !name.ends_with(".png") || !bytes.starts_with(b"\x89PNG") {
        return Err("só imagens PNG".into());
    }
    write_report(&app, name, bytes)
}

/// Abre um relatório salvo no programa padrão ou mostra o arquivo na pasta.
#[tauri::command]
fn open_report(app: AppHandle, path: String, reveal: bool) -> Result<(), String> {
    let path = PathBuf::from(path);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if path.parent() != Some(report_dir(&app)?.as_path()) || !is_report_name(name) {
        return Err("caminho fora da pasta de relatórios".into());
    }
    let opener = app.opener();
    let result = if reveal {
        opener.reveal_item_in_dir(&path)
    } else {
        opener.open_path(path.to_string_lossy(), None::<&str>)
    };
    result.map_err(|e| e.to_string())
}

/// Usado pelo binário `silicio-helper`.
pub use hw::privileged::read as privileged_read;

/// No Windows só a tabela SMBIOS, que não exige administrador.
#[cfg(windows)]
#[tauri::command]
async fn privileged_info() -> Result<PrivilegedInfo, String> {
    off_main(hw::privileged::read).await
}

/// Roda o `silicio-helper` como root via pkexec (o polkit pede a senha).
#[cfg(not(windows))]
#[tauri::command]
async fn privileged_info() -> Result<PrivilegedInfo, String> {
    off_main(|| {
        // Instalado, o helper fica em /usr/lib/silicio (caminho da política do polkit);
        // em desenvolvimento, ao lado do executável.
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let candidates = [
            "/usr/lib/silicio/silicio-helper".into(),
            exe.with_file_name("silicio-helper"),
        ];
        let helper = candidates.iter().find(|p| p.exists()).ok_or(
            "helper não encontrado. Ele vem nos pacotes .deb e .rpm; o AppImage não o inclui.",
        )?;
        let out = Command::new("pkexec")
            .arg(helper)
            .output()
            .map_err(|e| format!("pkexec: {e}"))?;
        match out.status.code() {
            Some(0) => serde_json::from_slice(&out.stdout)
                .map_err(|e| format!("resposta inválida do helper: {e}")),
            Some(126) => Err("Autenticação cancelada.".into()),
            Some(127) => Err("Sem autorização para executar o helper.".into()),
            _ => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        }
    })
    .await?
}

/// `--smoke-test`: abre a janela, passa pelas abas (Ctrl+1…9 e Ctrl+0) e fecha.
/// Usado pelo teste `tests/smoke.rs` para garantir que o app abre de verdade.
fn smoke_test(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(4));
        for tab in [1, 2, 3, 4, 5, 6, 7, 8, 9, 0] {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval(format!(
                    "window.dispatchEvent(new KeyboardEvent('keydown', {{ key: '{tab}', ctrlKey: true }}))"
                ));
            }
            std::thread::sleep(Duration::from_secs(3));
        }
        app.exit(0);
    });
}

/// Arquivo onde falhas fatais são registradas. No Windows o app não tem console,
/// então sem isso um crash fecha a janela sem deixar rastro.
pub fn crash_log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("silicio-crash.log")
}

fn install_crash_log() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = format!(
            "Silício {} ({} {})\n{info}\n\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::backtrace::Backtrace::force_capture()
        );
        let _ = std::fs::write(crash_log_path(), report);
        default(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_crash_log();
    let has_flag = |flag: &str| std::env::args().any(|a| a == flag);

    // `silicio --json` imprime o relatório e sai, sem abrir janela.
    if has_flag("--json") {
        let gpus = GpuSampler::new();
        let gpu_list = gpus.info();
        let report = serde_json::json!({
            "cpu": cpu::read_static(),
            "board": board::read(),
            "memory": memory::read(),
            "spd": spd::read(),
            "storage": storage::read(),
            "battery": battery::read(),
            "gpu_live": gpus.live(),
            "peripherals": peripherals::read(),
            "upgrades": upgrade::read(&gpu_list),
            "gpu": gpu_list,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("relatório é serializável")
        );
        return;
    }

    // `silicio --bench` roda o benchmark (5 s single + 5 s multi) e imprime o resultado.
    if has_flag("--bench") {
        let single = bench::benchmark(1, 5);
        let multi = bench::benchmark(bench::available_threads(), 5);
        let report = serde_json::json!({
            "single": single,
            "multi": multi,
            "ratio": multi.score / single.score,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("resultado é serializável")
        );
        return;
    }

    let smoke = has_flag("--smoke-test");

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(LiveSampler::new()))
        .manage(Mutex::new(System::new()))
        .manage(Mutex::new(GpuSampler::new()))
        .manage(Mutex::new(Stress::default()))
        .setup(move |app| {
            if smoke {
                smoke_test(app.handle().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cpu_info,
            cpu_live,
            board_info,
            memory_info,
            memory_live,
            spd_info,
            storage_info,
            battery_info,
            peripherals_info,
            upgrade_info,
            privileged_info,
            gpu_info,
            gpu_live,
            bench_run,
            stress_start,
            stress_stop,
            stress_status,
            system_info,
            save_report,
            save_image,
            open_report
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Silício");
}

#[cfg(test)]
mod tests {
    use super::is_report_name;

    #[test]
    fn so_aceita_nomes_de_relatorio() {
        assert!(is_report_name("Silicio-relatorio-2026-10-19.html"));
        assert!(is_report_name("Silicio-report.md"));
        assert!(is_report_name("Silicio-card-2026-10-19.png"));
        assert!(!is_report_name("Silicio-x.exe"));
        assert!(!is_report_name("../Silicio-x.html"));
        assert!(!is_report_name("Silicio-a/b.html"));
        assert!(!is_report_name("outro.html"));
    }
}
