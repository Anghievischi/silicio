//! Sessão de jogo: grava processador, placa de vídeo e memória a cada segundo
//! enquanto a pessoa joga e depois diz o que limitou o desempenho.
//!
//! A gravação roda num thread próprio, então continua com a janela minimizada
//! ou em outra aba. Não mede FPS: no Linux isso exigiria entrar no processo do
//! jogo, e no Windows ler os eventos de quadro (ETW) pede administrador.
//! O backend devolve códigos e números; a interface monta as frases.

use crate::hw::battery;
use crate::hw::cpu::LiveSampler;
use crate::hw::gpu::{Gpu, GpuLive};
use crate::hw::memory;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;

/// Seis horas de gravação; depois disso ela para sozinha.
const MAX_SAMPLES: usize = 6 * 3600;
/// Segundos de jogo necessários para um veredito.
const MIN_LOAD: usize = 30;
/// Abaixo disso o laudo sai, mas com a dica de gravar mais.
const GOOD_LOAD: usize = 120;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Sample {
    /// Segundos desde o início.
    pub t: u32,
    /// Uso total do processador (%).
    pub cpu_usage: f32,
    /// Thread mais ocupada (%): um jogo preso no processador costuma travar uma só.
    pub thread_max: f32,
    /// Clock do núcleo mais rápido: a média cai sozinha quando só um núcleo trabalha.
    pub cpu_mhz: u32,
    pub cpu_temp: Option<f32>,
    /// Placa de vídeo principal: a dedicada, se houver.
    pub gpu_usage: Option<f32>,
    pub gpu_mhz: Option<u32>,
    pub gpu_temp: Option<f32>,
    pub gpu_power: Option<f32>,
    /// Limite de clock informado pelo driver: "thermal" ou "power" (só NVIDIA).
    pub gpu_limit: Option<&'static str>,
    pub vram_used: Option<u64>,
    /// Uso da integrada quando também há uma dedicada.
    pub igpu_usage: Option<f32>,
    pub ram_used: u64,
    pub swap_used: u64,
    /// Lido a cada 10 s; nos outros segundos repete o último valor.
    pub on_battery: Option<bool>,
}

/// O que se sabe do computador além das amostras.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Context {
    pub gpu_name: Option<String>,
    /// A placa principal é dedicada.
    pub dedicated: bool,
    /// Há uma integrada além da dedicada (notebook híbrido, desktop com APU).
    pub hybrid: bool,
    pub vram_total: Option<u64>,
    pub ram_total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub seconds: u32,
    /// Segundos com jogo rodando, os únicos usados na análise.
    pub load_seconds: u32,
    /// O limite mais frequente ("gpu", "cpu", "vram", "ram", "thermal" ou "capped"),
    /// "balanced" quando nenhum passa de 40% do tempo, ou "idle" sem tempo de jogo.
    pub verdict: &'static str,
    /// Parte do tempo de jogo em cada limite, da maior para a menor.
    pub limits: Vec<Limit>,
    pub findings: Vec<Finding>,
    pub stats: Stats,
    pub context: Context,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Limit {
    pub code: &'static str,
    pub pct: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// "info", "warning" ou "problem".
    pub severity: &'static str,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "kebab-case")]
pub enum Kind {
    /// O jogo rodou na integrada e a dedicada ficou parada.
    WrongGpu,
    /// Clock do processador caiu sob carga. `cause`: "heat", "power" ou "unknown".
    CpuThrottle {
        peak_mhz: u32,
        low_mhz: u32,
        seconds: u32,
        first_at: u32,
        cause: &'static str,
        temp_max: Option<f32>,
    },
    CpuHot {
        temp_max: f32,
        seconds: u32,
    },
    GpuThrottle {
        peak_mhz: u32,
        low_mhz: u32,
        seconds: u32,
        first_at: u32,
        cause: &'static str,
        temp_max: Option<f32>,
        /// A causa veio do driver, não da temperatura.
        reported: bool,
    },
    /// O driver segurou o clock por calor, mesmo sem queda visível durante a sessão.
    GpuThermalLimit {
        seconds: u32,
        temp_max: Option<f32>,
    },
    GpuHot {
        temp_max: f32,
        seconds: u32,
    },
    /// VRAM no limite: texturas vão para a RAM e o jogo engasga.
    VramFull {
        peak_bytes: u64,
        total_bytes: u64,
    },
    RamFull {
        peak_bytes: u64,
        total_bytes: u64,
        swap_growth_bytes: u64,
    },
    OnBattery {
        seconds: u32,
    },
    ShortSession {
        load_seconds: u32,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Stat {
    pub avg: f64,
    pub max: f64,
}

/// Média e pico de cada leitura durante o tempo de jogo.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub cpu_usage: Option<Stat>,
    pub thread_max: Option<Stat>,
    pub cpu_mhz: Option<Stat>,
    pub cpu_temp: Option<Stat>,
    pub gpu_usage: Option<Stat>,
    pub gpu_mhz: Option<Stat>,
    pub gpu_temp: Option<Stat>,
    pub gpu_power: Option<Stat>,
    pub vram_used: Option<Stat>,
    pub ram_used: Option<Stat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStatus {
    pub running: bool,
    pub seconds: u32,
    pub samples: usize,
    pub last: Option<Sample>,
    /// Já existe um laudo da última sessão.
    pub has_report: bool,
}

/// Gravador mantido pelo Tauri entre chamadas.
#[derive(Default)]
pub struct Session {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<Sample>>>,
    started: Option<Instant>,
    context: Context,
    report: Option<Report>,
}

impl Session {
    /// Começa a gravar. `gpu_live` lê as placas de vídeo (o app compartilha a NVML).
    pub fn start<F>(&mut self, gpus: &[Gpu], gpu_live: F)
    where
        F: Fn() -> Vec<GpuLive> + Send + 'static,
    {
        if self.started.is_some() {
            return;
        }
        let main = gpus
            .iter()
            .find(|g| g.integrated == Some(false))
            .or_else(|| gpus.first());
        let main_id = main.map(|g| g.id.clone());
        let dedicated = main.is_some_and(|g| g.integrated == Some(false));
        let igpu_id = gpus
            .iter()
            .find(|g| g.integrated == Some(true))
            .filter(|_| dedicated)
            .map(|g| g.id.clone());

        let mut sys = System::new();
        let ram_total = memory::read_live(&mut sys).total_bytes;
        self.context = Context {
            gpu_name: main.and_then(|g| g.name.clone()),
            dedicated,
            hybrid: igpu_id.is_some(),
            vram_total: main.and_then(|g| g.vram_bytes).filter(|_| dedicated),
            ram_total,
        };
        self.stop = Arc::new(AtomicBool::new(false));
        self.samples = Arc::new(Mutex::new(Vec::new()));
        self.report = None;
        let started = Instant::now();
        self.started = Some(started);

        let (stop, samples) = (self.stop.clone(), self.samples.clone());
        thread::spawn(move || {
            // Amostrador próprio: o uso de CPU é a diferença desde a leitura anterior,
            // e as abas abertas também leem o delas.
            let mut cpu = LiveSampler::new();
            cpu.sample();
            let mut on_battery = None;
            let mut n = 0usize;
            while !stop.load(Ordering::Relaxed) && n < MAX_SAMPLES {
                thread::sleep(Duration::from_secs(1));
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let c = cpu.sample();
                let live = gpu_live();
                let find = |id: &Option<String>| live.iter().find(|g| Some(&g.id) == id.as_ref());
                let gpu = find(&main_id);
                let mem = memory::read_live(&mut sys);
                if n.is_multiple_of(10) {
                    on_battery = battery::read()
                        .first()
                        .map(|b| b.status == "discharging" || b.ac_online == Some(false));
                }
                let sample = Sample {
                    t: started.elapsed().as_secs() as u32,
                    cpu_usage: c.usage,
                    thread_max: c.threads.iter().map(|t| t.usage).fold(0.0, f32::max),
                    cpu_mhz: c.peak_mhz as u32,
                    cpu_temp: c.temperature_c,
                    gpu_usage: gpu.and_then(|g| g.usage).map(|u| u as f32),
                    gpu_mhz: gpu.and_then(|g| g.core_mhz),
                    gpu_temp: gpu.and_then(|g| g.temperature_c),
                    gpu_power: gpu.and_then(|g| g.power_w),
                    gpu_limit: gpu.and_then(|g| g.limit),
                    vram_used: gpu.and_then(|g| g.vram_used_bytes),
                    igpu_usage: find(&igpu_id).and_then(|g| g.usage).map(|u| u as f32),
                    ram_used: mem.used_bytes,
                    swap_used: mem.swap_used_bytes,
                    on_battery,
                };
                samples
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(sample);
                n += 1;
            }
        });
    }

    /// Para a gravação e devolve o laudo.
    pub fn stop(&mut self) -> Option<Report> {
        if self.started.take().is_some() {
            self.stop.store(true, Ordering::Relaxed);
            let samples =
                std::mem::take(&mut *self.samples.lock().unwrap_or_else(|e| e.into_inner()));
            self.report = Some(analyze(samples, self.context.clone()));
        }
        self.report.clone()
    }

    pub fn status(&self) -> SessionStatus {
        let samples = self.samples.lock().unwrap_or_else(|e| e.into_inner());
        SessionStatus {
            running: self.started.is_some(),
            seconds: self.started.map_or(0, |s| s.elapsed().as_secs() as u32),
            samples: samples.len(),
            last: samples.last().cloned().filter(|_| self.started.is_some()),
            has_report: self.report.is_some(),
        }
    }

    pub fn report(&self) -> Option<Report> {
        self.report.clone()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Segundo com jogo rodando: placa de vídeo trabalhando ou uma thread ocupada.
fn under_load(s: &Sample) -> bool {
    s.gpu_usage.is_some_and(|u| u >= 40.0)
        || s.igpu_usage.is_some_and(|u| u >= 40.0)
        || s.thread_max >= 60.0
}

fn stat(values: impl Iterator<Item = f64>) -> Option<Stat> {
    let (mut sum, mut max, mut n) = (0.0, f64::MIN, 0);
    for v in values {
        sum += v;
        max = max.max(v);
        n += 1;
    }
    (n > 0).then(|| Stat {
        avg: sum / n as f64,
        max,
    })
}

/// Valor no percentil `p` (0–1).
fn percentile(mut values: Vec<f64>, p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let i = ((values.len() - 1) as f64 * p).round() as usize;
    Some(values[i])
}

fn median(values: Vec<f64>) -> Option<f64> {
    percentile(values, 0.5)
}

fn pct(part: usize, total: usize) -> u8 {
    (part as f64 / total.max(1) as f64 * 100.0).round() as u8
}

/// Momentos em que o clock ficou mais de 20% abaixo do pico com o componente ocupado.
struct Throttle {
    peak: f64,
    low: f64,
    count: usize,
    first_at: u32,
    temp_max: Option<f32>,
    /// Motivo que o driver informou na maior parte dos momentos lentos.
    reported: Option<&'static str>,
}

fn throttle(
    load: &[&Sample],
    clock: impl Fn(&Sample) -> Option<f64>,
    busy: impl Fn(&Sample) -> bool,
    temp: impl Fn(&Sample) -> Option<f32>,
    reason: impl Fn(&Sample) -> Option<&'static str>,
) -> Option<Throttle> {
    let busy: Vec<_> = load.iter().copied().filter(|s| busy(s)).collect();
    // O pico é o percentil 95, para um segundo de boost isolado não contar.
    let peak = percentile(busy.iter().filter_map(|s| clock(s)).collect(), 0.95)?;
    let slow: Vec<_> = busy
        .iter()
        .filter(|s| clock(s).is_some_and(|c| c < peak * 0.8))
        .collect();
    if slow.len() < 10.max(busy.len() / 10) {
        return None;
    }
    Some(Throttle {
        peak,
        low: median(slow.iter().filter_map(|s| clock(s)).collect())?,
        count: slow.len(),
        first_at: slow[0].t,
        temp_max: slow.iter().filter_map(|s| temp(s)).reduce(f32::max),
        reported: ["thermal", "power"]
            .into_iter()
            .map(|r| (r, slow.iter().filter(|s| reason(s) == Some(r)).count()))
            .max_by_key(|&(_, n)| n)
            .filter(|&(_, n)| n * 2 >= slow.len())
            .map(|(r, _)| r),
    })
}

/// O que limitou um segundo de jogo, na ordem em que um limite esconde o outro.
/// Classificar segundo a segundo e mostrar a distribuição vem do ResetExplode
/// (github.com/GustavoReset/ResetExplode, licença MIT).
fn limiter(s: &Sample, gpu: Option<f32>, vram_total: Option<u64>, ram_total: u64) -> &'static str {
    let full = |used: u64, total: u64, pct: f64| total > 0 && used as f64 >= total as f64 * pct;
    if vram_total.is_some_and(|t| s.vram_used.is_some_and(|v| full(v, t, 0.97))) {
        "vram"
    } else if s.gpu_limit == Some("thermal") || s.cpu_temp.is_some_and(|t| t >= 95.0) {
        "thermal"
    } else if gpu.is_some_and(|u| u >= 90.0) {
        "gpu"
    } else if s.thread_max >= 85.0 {
        "cpu"
    } else if full(s.ram_used, ram_total, 0.92) {
        "ram"
    } else {
        "capped"
    }
}

/// Calor quando a temperatura nos momentos lentos passou de `hot`; energia quando ficou longe.
fn cause(temp_max: Option<f32>, hot: f32) -> &'static str {
    match temp_max {
        Some(t) if t >= hot => "heat",
        Some(t) if t < hot - 10.0 => "power",
        _ => "unknown",
    }
}

pub fn analyze(samples: Vec<Sample>, context: Context) -> Report {
    let load: Vec<&Sample> = samples.iter().filter(|s| under_load(s)).collect();
    let n = load.len();
    let mut findings = Vec::new();
    let mut push = |severity, kind| findings.push(Finding { severity, kind });

    // Notebook híbrido com a dedicada parada enquanto a integrada trabalha.
    let avg = |f: &dyn Fn(&Sample) -> Option<f32>| {
        stat(load.iter().filter_map(|s| f(s)).map(f64::from)).map(|s| s.avg)
    };
    let wrong_gpu = context.hybrid
        && n >= MIN_LOAD
        && avg(&|s| s.gpu_usage).is_some_and(|u| u < 10.0)
        && avg(&|s| s.igpu_usage).is_some_and(|u| u >= 60.0);
    // Uso da placa que de fato rodou o jogo.
    let gpu = |s: &Sample| if wrong_gpu { s.igpu_usage } else { s.gpu_usage };

    // A VRAM só conta na dedicada: a integrada tem uma fatia fixa que vive cheia.
    let vram_total = context
        .vram_total
        .filter(|_| context.dedicated && !wrong_gpu);
    let mut limits: Vec<Limit> = ["gpu", "cpu", "vram", "ram", "thermal", "capped"]
        .into_iter()
        .map(|code| Limit {
            code,
            pct: pct(
                load.iter()
                    .filter(|s| limiter(s, gpu(s), vram_total, context.ram_total) == code)
                    .count(),
                n,
            ),
        })
        .filter(|l| l.pct > 0)
        .collect();
    limits.sort_by_key(|l| std::cmp::Reverse(l.pct));
    let has_gpu_usage = load.iter().any(|s| gpu(s).is_some());
    let verdict = match limits.first() {
        _ if n < MIN_LOAD || !has_gpu_usage => "idle",
        Some(top) if top.pct >= 40 => top.code,
        _ => "balanced",
    };

    if wrong_gpu {
        push("problem", Kind::WrongGpu);
    }

    if n >= MIN_LOAD {
        let cpu_t = throttle(
            &load,
            |s| Some(s.cpu_mhz as f64).filter(|&m| m > 0.0),
            |s| s.thread_max >= 60.0,
            |s| s.cpu_temp,
            |_| None,
        );
        let hot_cpu: Vec<f32> = load
            .iter()
            .filter_map(|s| s.cpu_temp)
            .filter(|&t| t >= 95.0)
            .collect();
        if let Some(t) = &cpu_t {
            push(
                "warning",
                Kind::CpuThrottle {
                    peak_mhz: t.peak as u32,
                    low_mhz: t.low as u32,
                    seconds: t.count as u32,
                    first_at: t.first_at,
                    cause: cause(t.temp_max, 90.0),
                    temp_max: t.temp_max,
                },
            );
        }
        if hot_cpu.len() >= 10
            && !cpu_t
                .as_ref()
                .is_some_and(|t| cause(t.temp_max, 90.0) == "heat")
        {
            push(
                "warning",
                Kind::CpuHot {
                    temp_max: hot_cpu.iter().copied().fold(0.0, f32::max),
                    seconds: hot_cpu.len() as u32,
                },
            );
        }

        let gpu_t = throttle(
            &load,
            |s| s.gpu_mhz.map(f64::from).filter(|&m| m > 0.0),
            |s| s.gpu_usage.is_some_and(|u| u >= 90.0),
            |s| s.gpu_temp,
            |s| s.gpu_limit,
        )
        .filter(|_| !wrong_gpu);
        // O motivo do driver vale mais que a temperatura: limite de potência a 85 °C é potência.
        let gpu_cause = |t: &Throttle| match t.reported {
            Some("thermal") => "heat",
            Some(_) => "power",
            None => cause(t.temp_max, 83.0),
        };
        let thermal_limit: Vec<&&Sample> = load
            .iter()
            .filter(|s| s.gpu_limit == Some("thermal"))
            .collect();
        let hot_gpu: Vec<f32> = load
            .iter()
            .filter_map(|s| s.gpu_temp)
            .filter(|&t| t >= 87.0)
            .collect();
        if let Some(t) = &gpu_t {
            push(
                "warning",
                Kind::GpuThrottle {
                    peak_mhz: t.peak as u32,
                    low_mhz: t.low as u32,
                    seconds: t.count as u32,
                    first_at: t.first_at,
                    cause: gpu_cause(t),
                    temp_max: t.temp_max,
                    reported: t.reported.is_some(),
                },
            );
        }
        let heat_explained = gpu_t.as_ref().is_some_and(|t| gpu_cause(t) == "heat");
        let thermal_reported = !wrong_gpu && !heat_explained && thermal_limit.len() >= 10;
        if thermal_reported {
            push(
                "warning",
                Kind::GpuThermalLimit {
                    seconds: thermal_limit.len() as u32,
                    temp_max: thermal_limit
                        .iter()
                        .filter_map(|s| s.gpu_temp)
                        .reduce(f32::max),
                },
            );
        }
        if hot_gpu.len() >= 10 && !heat_explained && !thermal_reported {
            push(
                "warning",
                Kind::GpuHot {
                    temp_max: hot_gpu.iter().copied().fold(0.0, f32::max),
                    seconds: hot_gpu.len() as u32,
                },
            );
        }

        if let Some(total) = vram_total {
            let full = load
                .iter()
                .filter(|s| s.vram_used.is_some_and(|v| v as f64 >= total as f64 * 0.95))
                .count();
            if full >= 5.max(n / 20) {
                push(
                    "warning",
                    Kind::VramFull {
                        peak_bytes: load.iter().filter_map(|s| s.vram_used).max().unwrap_or(0),
                        total_bytes: total,
                    },
                );
            }
        }

        let ram_total = context.ram_total.max(1);
        let ram_full = load
            .iter()
            .filter(|s| s.ram_used as f64 >= ram_total as f64 * 0.92)
            .count();
        let swap_start = samples.first().map_or(0, |s| s.swap_used);
        let swap_growth = samples
            .iter()
            .map(|s| s.swap_used)
            .max()
            .unwrap_or(0)
            .saturating_sub(swap_start);
        if ram_full >= 5 || swap_growth >= 512 << 20 {
            push(
                "warning",
                Kind::RamFull {
                    peak_bytes: load.iter().map(|s| s.ram_used).max().unwrap_or(0),
                    total_bytes: context.ram_total,
                    swap_growth_bytes: swap_growth,
                },
            );
        }

        let battery = load.iter().filter(|s| s.on_battery == Some(true)).count();
        if battery >= 10 {
            push(
                "warning",
                Kind::OnBattery {
                    seconds: battery as u32,
                },
            );
        }

        if n < GOOD_LOAD {
            push(
                "info",
                Kind::ShortSession {
                    load_seconds: n as u32,
                },
            );
        }
    }

    findings.sort_by_key(|f| match f.severity {
        "problem" => 0,
        "warning" => 1,
        _ => 2,
    });

    let f = |get: fn(&Sample) -> Option<f64>| stat(load.iter().filter_map(|s| get(s)));
    let stats = Stats {
        cpu_usage: f(|s| Some(s.cpu_usage as f64)),
        thread_max: f(|s| Some(s.thread_max as f64)),
        cpu_mhz: f(|s| Some(s.cpu_mhz as f64).filter(|&m| m > 0.0)),
        cpu_temp: f(|s| s.cpu_temp.map(f64::from)),
        gpu_usage: f(|s| s.gpu_usage.map(f64::from)),
        gpu_mhz: f(|s| s.gpu_mhz.map(f64::from)),
        gpu_temp: f(|s| s.gpu_temp.map(f64::from)),
        gpu_power: f(|s| s.gpu_power.map(f64::from)),
        vram_used: f(|s| s.vram_used.map(|v| v as f64)),
        ram_used: f(|s| Some(s.ram_used as f64)),
    };

    Report {
        seconds: samples.last().map_or(0, |s| s.t),
        load_seconds: n as u32,
        verdict,
        limits,
        findings,
        stats,
        context,
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn context(dedicated: bool, hybrid: bool) -> Context {
        Context {
            gpu_name: Some("GeForce RTX 4060".into()),
            dedicated,
            hybrid,
            vram_total: dedicated.then_some(8 * GIB),
            ram_total: 16 * GIB,
        }
    }

    /// `n` segundos de jogo com a placa em `gpu`% e a thread mais ocupada em `thread`%.
    fn game(n: u32, gpu: f32, thread: f32) -> Vec<Sample> {
        (0..n)
            .map(|t| Sample {
                t,
                cpu_usage: thread / 3.0,
                thread_max: thread,
                cpu_mhz: 4500,
                cpu_temp: Some(75.0),
                gpu_usage: Some(gpu),
                gpu_mhz: Some(2500),
                gpu_temp: Some(70.0),
                gpu_power: Some(110.0),
                gpu_limit: None,
                vram_used: Some(5 * GIB),
                igpu_usage: None,
                ram_used: 9 * GIB,
                swap_used: 0,
                on_battery: Some(false),
            })
            .collect()
    }

    fn kinds(r: &Report) -> Vec<&Kind> {
        r.findings.iter().map(|f| &f.kind).collect()
    }

    #[test]
    fn placa_de_video_no_limite() {
        let r = analyze(game(300, 98.0, 55.0), context(true, false));
        assert_eq!(r.verdict, "gpu");
        assert_eq!(
            r.limits,
            vec![Limit {
                code: "gpu",
                pct: 100
            }]
        );
        assert!(r.findings.is_empty(), "{:?}", r.findings);
        assert_eq!(r.load_seconds, 300);
    }

    #[test]
    fn processador_no_limite() {
        let r = analyze(game(300, 60.0, 99.0), context(true, false));
        assert_eq!(r.verdict, "cpu");
        assert_eq!(
            r.limits,
            vec![Limit {
                code: "cpu",
                pct: 100
            }]
        );
    }

    #[test]
    fn jogo_com_limite_de_fps() {
        let r = analyze(game(300, 55.0, 50.0), context(true, false));
        assert_eq!(r.verdict, "capped");
    }

    #[test]
    fn pouco_tempo_de_jogo() {
        let mut samples = game(20, 98.0, 50.0);
        // Minutos no menu do Silício antes de abrir o jogo não contam.
        samples.extend(game(200, 3.0, 10.0));
        let r = analyze(samples, context(true, false));
        assert_eq!(r.verdict, "idle");
        assert_eq!(r.load_seconds, 20);
        assert!(r.findings.is_empty());

        let r = analyze(game(60, 98.0, 50.0), context(true, false));
        assert_eq!(kinds(&r), vec![&Kind::ShortSession { load_seconds: 60 }]);
    }

    #[test]
    fn jogo_na_integrada_do_notebook() {
        let mut samples = game(200, 2.0, 70.0);
        for s in &mut samples {
            s.igpu_usage = Some(97.0);
        }
        let r = analyze(samples, context(true, true));
        assert_eq!(r.findings[0].kind, Kind::WrongGpu);
        assert_eq!(r.findings[0].severity, "problem");
        // O veredito usa a placa que rodou o jogo.
        assert_eq!(r.verdict, "gpu");
    }

    #[test]
    fn throttling_por_calor_na_cpu_e_na_gpu() {
        let mut samples = game(300, 98.0, 90.0);
        for s in samples.iter_mut().skip(120) {
            s.cpu_mhz = 3000;
            s.cpu_temp = Some(97.0);
            s.gpu_mhz = Some(1800);
            s.gpu_temp = Some(88.0);
        }
        let r = analyze(samples, context(true, false));
        let k = kinds(&r);
        assert!(k.iter().any(|k| matches!(
            k,
            Kind::CpuThrottle {
                cause: "heat",
                low_mhz: 3000,
                first_at: 120,
                ..
            }
        )));
        assert!(k.iter().any(|k| matches!(
            k,
            Kind::GpuThrottle {
                cause: "heat",
                peak_mhz: 2500,
                ..
            }
        )));
        // Com o throttling já explicado pelo calor, não repete o aviso de temperatura.
        assert!(
            !k.iter()
                .any(|k| matches!(k, Kind::CpuHot { .. } | Kind::GpuHot { .. }))
        );
    }

    #[test]
    fn queda_de_clock_sem_calor_e_limite_de_energia() {
        let mut samples = game(300, 98.0, 90.0);
        for s in samples.iter_mut().skip(150) {
            s.cpu_mhz = 2800;
        }
        let r = analyze(samples, context(true, false));
        assert!(
            kinds(&r)
                .iter()
                .any(|k| matches!(k, Kind::CpuThrottle { cause: "power", .. }))
        );
    }

    #[test]
    fn vram_e_ram_cheias_na_bateria() {
        let mut samples = game(300, 98.0, 60.0);
        for s in samples.iter_mut().skip(100) {
            s.vram_used = Some(8 * GIB - (100 << 20));
            s.ram_used = 15 * GIB;
            s.swap_used = GIB;
            s.on_battery = Some(true);
        }
        let r = analyze(samples, context(true, false));
        let k = kinds(&r);
        assert!(k.contains(&&Kind::VramFull {
            peak_bytes: 8 * GIB - (100 << 20),
            total_bytes: 8 * GIB
        }));
        assert!(k.contains(&&Kind::RamFull {
            peak_bytes: 15 * GIB,
            total_bytes: 16 * GIB,
            swap_growth_bytes: GIB
        }));
        assert!(k.contains(&&Kind::OnBattery { seconds: 200 }));
    }

    #[test]
    fn motivo_informado_pelo_driver() {
        // Queda de clock a 85 °C, mas o driver diz que é limite de potência.
        let mut samples = game(300, 98.0, 60.0);
        for s in samples.iter_mut().skip(150) {
            s.gpu_mhz = Some(1800);
            s.gpu_temp = Some(85.0);
            s.gpu_limit = Some("power");
        }
        let r = analyze(samples, context(true, false));
        assert!(kinds(&r).iter().any(|k| matches!(
            k,
            Kind::GpuThrottle {
                cause: "power",
                reported: true,
                ..
            }
        )));

        // Clock estável, mas o driver segurou por calor a sessão toda.
        let mut samples = game(300, 98.0, 60.0);
        for s in &mut samples {
            s.gpu_limit = Some("thermal");
            s.gpu_temp = Some(88.0);
        }
        let r = analyze(samples, context(true, false));
        assert_eq!(
            kinds(&r),
            vec![&Kind::GpuThermalLimit {
                seconds: 300,
                temp_max: Some(88.0)
            }]
        );
        // O calor esconde o limite da GPU em cada segundo.
        assert_eq!(r.verdict, "thermal");
    }

    #[test]
    fn distribuicao_do_tempo_por_limite() {
        let mut samples = game(100, 98.0, 50.0);
        samples.extend(game(60, 70.0, 95.0));
        samples.extend(game(40, 55.0, 50.0));
        let r = analyze(samples, context(true, false));
        assert_eq!(
            r.limits,
            vec![
                Limit {
                    code: "gpu",
                    pct: 50
                },
                Limit {
                    code: "cpu",
                    pct: 30
                },
                Limit {
                    code: "capped",
                    pct: 20
                },
            ]
        );
        assert_eq!(r.verdict, "gpu");

        // Nenhum limite chega a 40%: carga equilibrada.
        let mut samples = game(70, 98.0, 50.0);
        samples.extend(game(70, 70.0, 95.0));
        samples.extend(game(60, 55.0, 50.0));
        assert_eq!(analyze(samples, context(true, false)).verdict, "balanced");
    }

    #[test]
    fn vram_da_integrada_nao_conta() {
        let mut samples = game(300, 98.0, 60.0);
        for s in &mut samples {
            s.vram_used = Some(512 << 20);
        }
        let mut ctx = context(false, false);
        ctx.vram_total = Some(512 << 20);
        assert!(analyze(samples, ctx).findings.is_empty());
    }

    #[test]
    fn gravacao_para_quando_pedido() {
        let mut s = Session::default();
        s.start(&[], Vec::new);
        thread::sleep(Duration::from_millis(1500));
        let status = s.status();
        assert!(status.running);
        assert!(status.samples >= 1);
        let report = s.stop().unwrap();
        assert_eq!(report.verdict, "idle");
        assert!(!s.status().running);
        assert!(s.status().has_report);
    }
}
