//! Helper com privilégio de root, iniciado pelo app via `pkexec` no Linux: lê
//! MSR e SMBIOS e imprime o resultado em JSON.

fn main() {
    let info = silicio_lib::privileged_read();
    println!(
        "{}",
        serde_json::to_string(&info).expect("PrivilegedInfo é serializável")
    );
}
