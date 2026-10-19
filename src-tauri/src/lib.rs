mod bench;
mod hw;

use bench::{BenchResult, Stress, StressStatus};
use hw::board::{self, BoardInfo};
use hw::cpu::{self, CpuInfo, CpuLive, LiveSampler};
use hw::gpu::{Gpu, GpuLive, GpuSampler};
use hw::memory::{self, MemoryInfo, MemoryLive};
use hw::privileged::PrivilegedInfo;
use hw::spd::{self, SpdModule};
use std::process::Command;
use std::sync::Mutex;
use sysinfo::System;

#[tauri::command]
fn cpu_info() -> CpuInfo {
    cpu::read_static()
}

#[tauri::command]
fn cpu_live(sampler: tauri::State<'_, Mutex<LiveSampler>>) -> CpuLive {
    sampler.lock().unwrap_or_else(|e| e.into_inner()).sample()
}

#[tauri::command]
fn board_info() -> BoardInfo {
    board::read()
}

#[tauri::command]
fn memory_info() -> MemoryInfo {
    memory::read()
}

#[tauri::command]
fn memory_live(sys: tauri::State<'_, Mutex<System>>) -> MemoryLive {
    memory::read_live(&mut sys.lock().unwrap_or_else(|e| e.into_inner()))
}

#[tauri::command]
fn gpu_info(sampler: tauri::State<'_, Mutex<GpuSampler>>) -> Vec<Gpu> {
    sampler.lock().unwrap_or_else(|e| e.into_inner()).info()
}

#[tauri::command]
fn gpu_live(sampler: tauri::State<'_, Mutex<GpuSampler>>) -> Vec<GpuLive> {
    sampler.lock().unwrap_or_else(|e| e.into_inner()).live()
}

#[tauri::command]
fn spd_info() -> Vec<SpdModule> {
    spd::read()
}

/// Roda o benchmark fora da thread principal; `threads` vazio usa todas as threads.
#[tauri::command]
async fn bench_run(threads: Option<usize>, seconds: u64) -> Result<BenchResult, String> {
    let threads = threads.unwrap_or_else(bench::available_threads);
    let seconds = seconds.clamp(1, 60);
    tauri::async_runtime::spawn_blocking(move || bench::benchmark(threads, seconds))
        .await
        .map_err(|e| e.to_string())
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

/// Usado pelo binário `silicio-helper`.
pub use hw::privileged::read as privileged_read;

/// Roda o `silicio-helper` como root via pkexec (o polkit pede a senha).
#[tauri::command]
async fn privileged_info() -> Result<PrivilegedInfo, String> {
    tauri::async_runtime::spawn_blocking(|| {
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
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // `silicio --json` imprime o relatório e sai, sem abrir janela.
    if std::env::args().any(|a| a == "--json") {
        let gpus = GpuSampler::new();
        let report = serde_json::json!({
            "cpu": cpu::read_static(),
            "board": board::read(),
            "memory": memory::read(),
            "spd": spd::read(),
            "gpu": gpus.info(),
            "gpu_live": gpus.live(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("relatório é serializável")
        );
        return;
    }

    // `silicio --bench` roda o benchmark (5 s single + 5 s multi) e imprime o resultado.
    if std::env::args().any(|a| a == "--bench") {
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(LiveSampler::new()))
        .manage(Mutex::new(System::new()))
        .manage(Mutex::new(GpuSampler::new()))
        .manage(Mutex::new(Stress::default()))
        .invoke_handler(tauri::generate_handler![
            cpu_info,
            cpu_live,
            board_info,
            memory_info,
            memory_live,
            spd_info,
            privileged_info,
            gpu_info,
            gpu_live,
            bench_run,
            stress_start,
            stress_stop,
            stress_status
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Silício");
}
