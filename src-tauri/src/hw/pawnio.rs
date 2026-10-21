//! Acesso ao driver PawnIO (https://pawnio.eu), usado no Windows para ler
//! registradores do processador sem um driver próprio.
//!
//! O PawnIO executa módulos assinados; o Silício carrega o `AMDFamily17` ou o
//! `IntelMSR` (pasta `pawnio/`, LGPL-2.1) e chama as funções deles por IOCTL.
//! Abrir o dispositivo exige administrador.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::io::AsRawHandle;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentThread, ReleaseMutex, SetThreadAffinityMask, WaitForSingleObject,
};
use windows::core::w;

const DEVICE: &str = r"\\?\GLOBALROOT\Device\PawnIO";
// CTL_CODE(41394, função, METHOD_BUFFERED, FILE_ANY_ACCESS), de `pawnio_um.h`.
const IOCTL_LOAD_BINARY: u32 = 0xA1B2_2084;
const IOCTL_EXECUTE_FN: u32 = 0xA1B2_2104;
/// O nome da função vai num campo fixo de 32 bytes antes dos argumentos.
const FN_NAME_LENGTH: usize = 32;

const AMD_MODULE: &[u8] = include_bytes!("../../pawnio/AMDFamily17.bin");
const INTEL_MODULE: &[u8] = include_bytes!("../../pawnio/IntelMSR.bin");
pub struct PawnIo {
    file: File,
    /// Mutex global que ferramentas de hardware usam para não intercalar
    /// acessos ao espaço de configuração PCI (o SMN passa por ele).
    mutex: HANDLE,
}

#[derive(Debug)]
pub enum OpenError {
    NotInstalled,
    AccessDenied,
    Unsupported(String),
    Other(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotInstalled => write!(
                f,
                "o driver PawnIO não está instalado. Reinstale o Silício e aceite a instalação do driver, ou instale-o em https://pawnio.eu (`winget install namazso.PawnIO`), e tente de novo."
            ),
            Self::AccessDenied => write!(f, "o PawnIO exige administrador"),
            Self::Unsupported(cpu) => {
                write!(f, "processador {cpu} não suportado pelos módulos do PawnIO")
            }
            Self::Other(e) => write!(f, "PawnIO: {e}"),
        }
    }
}

impl PawnIo {
    pub fn open() -> Result<Self, OpenError> {
        // O módulo certo para o fabricante; ambos expõem `ioctl_read_msr`.
        let module = match raw_cpuid::CpuId::new()
            .get_vendor_info()
            .map(|v| v.as_str().to_string())
            .as_deref()
        {
            Some("AuthenticAMD") => AMD_MODULE,
            Some("GenuineIntel") => INTEL_MODULE,
            other => return Err(OpenError::Unsupported(other.unwrap_or("?").to_string())),
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEVICE)
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => OpenError::NotInstalled,
                io::ErrorKind::PermissionDenied => OpenError::AccessDenied,
                _ => OpenError::Other(e.to_string()),
            })?;
        let mutex = unsafe { CreateMutexW(None, false, w!("Global\\Access_PCI")) }
            .map_err(|e| OpenError::Other(e.to_string()))?;
        let io = Self { file, mutex };
        // O módulo recusa a carga (main() falha) em famílias que ele não conhece.
        io.ioctl(IOCTL_LOAD_BINARY, module, &mut [])
            .map_err(|e| OpenError::Unsupported(format!("({e})")))?;
        Ok(io)
    }

    fn ioctl(&self, code: u32, input: &[u8], output: &mut [u8]) -> io::Result<usize> {
        let mut returned = 0u32;
        unsafe {
            DeviceIoControl(
                HANDLE(self.file.as_raw_handle()),
                code,
                Some(input.as_ptr().cast()),
                input.len() as u32,
                (!output.is_empty()).then(|| output.as_mut_ptr().cast()),
                output.len() as u32,
                Some(&mut returned),
                None,
            )
        }
        .map_err(io::Error::other)?;
        Ok(returned as usize)
    }

    fn execute(&self, name: &str, input: &[u64], outputs: usize) -> io::Result<Vec<u64>> {
        let mut buf = vec![0u8; FN_NAME_LENGTH];
        buf[..name.len()].copy_from_slice(name.as_bytes());
        buf.extend(input.iter().flat_map(|v| v.to_le_bytes()));
        let mut out = vec![0u8; outputs * 8];
        let n = self.ioctl(IOCTL_EXECUTE_FN, &buf, &mut out)?;
        Ok(out[..n]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| u64::from_le_bytes(*c))
            .collect())
    }

    /// Lê um MSR no processador lógico `cpu` (a função roda no núcleo atual,
    /// então o thread é fixado nele durante a leitura).
    pub fn msr(&self, cpu: usize, reg: u64) -> io::Result<u64> {
        if cpu >= usize::BITS as usize {
            return Err(io::Error::other("processador fora do primeiro grupo"));
        }
        let thread = unsafe { GetCurrentThread() };
        let previous = unsafe { SetThreadAffinityMask(thread, 1 << cpu) };
        if previous == 0 {
            return Err(io::Error::last_os_error());
        }
        let value = self.execute("ioctl_read_msr", &[reg], 1);
        unsafe { SetThreadAffinityMask(thread, previous) };
        value?
            .first()
            .copied()
            .ok_or_else(|| io::Error::other("resposta vazia"))
    }

    /// Roda `f` segurando o mutex global, para outras ferramentas (HWiNFO,
    /// LibreHardwareMonitor…) não mexerem no par índice/dado no meio da leitura.
    pub fn exclusive<T>(&self, f: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
        let wait = unsafe { WaitForSingleObject(self.mutex, 1000) };
        if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
            return Err(io::Error::other("barramento ocupado por outro programa"));
        }
        let result = f();
        let _ = unsafe { ReleaseMutex(self.mutex) };
        result
    }

    /// Lê um registrador SMN (System Management Network) de CPUs AMD Zen.
    pub fn smn(&self, address: u32) -> io::Result<u32> {
        self.exclusive(|| {
            self.execute("ioctl_read_smn", &[address as u64], 1)?
                .first()
                .map(|v| *v as u32)
                .ok_or_else(|| io::Error::other("resposta vazia"))
        })
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.mutex) };
    }
}

// O handle do mutex só é usado com WaitForSingleObject/ReleaseMutex, que
// valem em qualquer thread.
unsafe impl Send for PawnIo {}
