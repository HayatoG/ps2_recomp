//! Utilitarios de sistema de arquivos: espaco livre em disco.

/// Bytes livres no volume que contem `path` (o diretorio precisa existir). `None` se nao der para
/// saber nesta plataforma; o chamador deve seguir em frente nesse caso.
#[cfg(windows)]
pub fn free_space(path: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: ponteiros validos para u64 e string UTF-16 terminada em zero.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
    (ok != 0).then_some(avail)
}

#[cfg(not(windows))]
pub fn free_space(_path: &std::path::Path) -> Option<u64> {
    None
}
