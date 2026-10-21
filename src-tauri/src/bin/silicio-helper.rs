//! Helper com privilégio de administrador.
//!
//! No Linux é iniciado pelo app via `pkexec`, sem argumentos: lê MSR e SMBIOS
//! e imprime o resultado em JSON. No Windows o app o inicia pelo UAC com
//! `--pipe <nome>`, e ele manda as leituras do PawnIO por esse named pipe
//! enquanto o app estiver aberto.

fn main() {
    #[cfg(all(windows, target_arch = "x86_64"))]
    if let [flag, pipe] = &std::env::args().skip(1).collect::<Vec<_>>()[..]
        && flag == "--pipe"
    {
        if let Err(e) = silicio_lib::privileged_serve(pipe) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }

    let info = silicio_lib::privileged_read();
    println!(
        "{}",
        serde_json::to_string(&info).expect("PrivilegedInfo é serializável")
    );
}
