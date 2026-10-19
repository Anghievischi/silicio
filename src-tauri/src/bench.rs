//! Benchmark de CPU e teste de estresse.
//!
//! A carga é a renderização de blocos do conjunto de Mandelbrot (ponto
//! flutuante) seguida de um hash xorshift do resultado (inteiros). Cada bloco
//! é uma "unidade"; a pontuação é o número de unidades por segundo.

use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const TILE: usize = 48;
const MAX_ITER: u32 = 200;

#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub threads: usize,
    pub seconds: f64,
    pub units: u64,
    pub score: f64,
}

/// Renderiza um bloco 48×48 do Mandelbrot e devolve um hash do resultado.
/// O índice desloca a região para que cada unidade faça um trabalho diferente.
fn work_unit(index: u64) -> u64 {
    let offset = (index % 64) as f64 * 0.002;
    let mut hash: u64 = 0x9E37_79B9_7F4A_7C15 ^ index;
    for py in 0..TILE {
        for px in 0..TILE {
            let cx = -0.75 + offset + (px as f64 / TILE as f64 - 0.5) * 0.4;
            let cy = 0.1 + (py as f64 / TILE as f64 - 0.5) * 0.4;
            let (mut x, mut y, mut i) = (0.0f64, 0.0f64, 0u32);
            while i < MAX_ITER && x * x + y * y <= 4.0 {
                let xt = x * x - y * y + cx;
                y = 2.0 * x * y + cy;
                x = xt;
                i += 1;
            }
            hash ^= i as u64;
            hash ^= hash << 13;
            hash ^= hash >> 7;
            hash ^= hash << 17;
        }
    }
    hash
}

/// Roda a carga em `threads` threads por `duration` e soma as unidades feitas.
fn run(threads: usize, duration: Duration, stop: &AtomicBool) -> (u64, f64) {
    let start = Instant::now();
    let total: u64 = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || {
                    let mut units = 0u64;
                    let mut index = t as u64 * 1_000_003;
                    while start.elapsed() < duration && !stop.load(Ordering::Relaxed) {
                        black_box(work_unit(black_box(index)));
                        units += 1;
                        index += 1;
                    }
                    units
                })
            })
            .collect();
        workers.into_iter().map(|w| w.join().unwrap_or(0)).sum()
    });
    (total, start.elapsed().as_secs_f64())
}

pub fn benchmark(threads: usize, seconds: u64) -> BenchResult {
    let threads = threads.max(1);
    let never = AtomicBool::new(false);
    let (units, elapsed) = run(threads, Duration::from_secs(seconds), &never);
    BenchResult {
        threads,
        seconds: elapsed,
        units,
        score: units as f64 / elapsed,
    }
}

pub fn available_threads() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Teste de estresse: carga em todas as threads até ser parado.
#[derive(Default)]
pub struct Stress {
    stop: Arc<AtomicBool>,
    units: Arc<AtomicU64>,
    started: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StressStatus {
    pub running: bool,
    pub seconds: f64,
    pub units: u64,
}

impl Stress {
    pub fn start(&mut self) {
        if self.started.is_some() {
            return;
        }
        self.stop = Arc::new(AtomicBool::new(false));
        self.units = Arc::new(AtomicU64::new(0));
        self.started = Some(Instant::now());
        for t in 0..available_threads() {
            let (stop, units) = (self.stop.clone(), self.units.clone());
            thread::spawn(move || {
                let mut index = t as u64 * 1_000_003;
                while !stop.load(Ordering::Relaxed) {
                    black_box(work_unit(black_box(index)));
                    units.fetch_add(1, Ordering::Relaxed);
                    index += 1;
                }
            });
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.started = None;
    }

    pub fn status(&self) -> StressStatus {
        StressStatus {
            running: self.started.is_some(),
            seconds: self
                .started
                .map(|s| s.elapsed().as_secs_f64())
                .unwrap_or(0.0),
            units: self.units.load(Ordering::Relaxed),
        }
    }
}

impl Drop for Stress {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unidade_e_deterministica() {
        assert_eq!(work_unit(7), work_unit(7));
        assert_ne!(work_unit(7), work_unit(8));
    }

    #[test]
    fn benchmark_curto_conta_unidades() {
        let r = benchmark(2, 1);
        assert_eq!(r.threads, 2);
        assert!(r.units > 0);
        assert!(r.seconds >= 1.0);
        assert!(r.score > 0.0);
    }

    #[test]
    fn estresse_para_quando_pedido() {
        let mut s = Stress::default();
        s.start();
        thread::sleep(Duration::from_millis(300));
        assert!(s.status().running);
        assert!(s.status().units > 0);
        s.stop();
        assert!(!s.status().running);
    }
}
