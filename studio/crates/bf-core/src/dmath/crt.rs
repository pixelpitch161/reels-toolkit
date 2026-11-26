//! The Windows C runtime's maths functions, called where they are: `ucrtbase.dll` from the Windows system folder,
//! loaded once at run time and asked for each function by name. Nothing of the runtime is copied into the engine;
//! the engine calls the very code Python calls on the reference PC, so it gets the very bits.
//!
//! The runtime chooses between two versions of most functions when it loads: one built on the processor's fused
//! multiply-add (the reference PC's) and one without it, for a processor that lacks the instruction. They differ in
//! the last place for a few arguments in a thousand (none for the logarithms), so a PC without fused multiply-add,
//! or a Windows that ships a different `ucrtbase.dll`, could give other bits. [`super::check`] finds that out and
//! says so in plain words; nothing here hides it.

use std::ffi::c_void;
use std::sync::OnceLock;

type One = unsafe extern "C" fn(f64) -> f64;
type Two = unsafe extern "C" fn(f64, f64) -> f64;
type Flag = unsafe extern "C" fn() -> i32;

/// The runtime's functions this engine calls.
pub(super) struct Crt {
    pub(super) sin: One,
    pub(super) cos: One,
    pub(super) tan: One,
    pub(super) asin: One,
    pub(super) acos: One,
    pub(super) atan: One,
    pub(super) atan2: Two,
    pub(super) exp: One,
    pub(super) log: One,
    pub(super) log10: One,
    pub(super) log2: One,
    pub(super) pow: Two,
    pub(super) tanh: One,
    pub(super) sinh: One,
    /// `_get_FMA3_enable`: whether the runtime took its fused multiply-add versions on this processor.
    pub(super) fma3: Option<Flag>,
}

#[allow(non_snake_case)]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

/// `LOAD_LIBRARY_SEARCH_SYSTEM32`: only the copy in the Windows system folder, never one beside the program.
const SYSTEM32_ONLY: u32 = 0x0000_0800;

/// The plain words for a Windows without the runtime (every Windows 10 and 11 has it).
const MISSING: &str = "This PC's Windows has no maths library (ucrtbase.dll in the Windows system folder), which \
                       BALLFIGHT Studio needs to compute fights the same way as the reference PC. Windows Update \
                       installs it.";

/// The runtime, loaded on the first call and kept for the life of the program.
#[inline(always)]
pub(super) fn crt() -> &'static Crt {
    static CRT: OnceLock<Crt> = OnceLock::new();
    CRT.get_or_init(load)
}

fn load() -> Crt {
    let name: Vec<u16> = "ucrtbase.dll\0".encode_utf16().collect();
    // SAFETY: `name` is a NUL-terminated wide string that outlives the call; the module is never freed, so every
    // address taken from it stays valid for the life of the program.
    let module = unsafe { LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), SYSTEM32_ONLY) };
    assert!(!module.is_null(), "{MISSING}");
    let find = |symbol: &[u8]| -> *mut c_void {
        debug_assert_eq!(symbol.last(), Some(&0));
        // SAFETY: `module` is a loaded module and `symbol` a NUL-terminated name.
        unsafe { GetProcAddress(module, symbol.as_ptr()) }
    };
    let need = |symbol: &[u8]| -> *mut c_void {
        let address = find(symbol);
        assert!(!address.is_null(), "{MISSING}");
        address
    };
    // SAFETY: each name is a C runtime function of exactly the signature it is cast to (C `double f(double)` and
    // `double f(double, double)`, `int f(void)`), and each address is non-null.
    unsafe {
        let one = |s: &[u8]| std::mem::transmute::<*mut c_void, One>(need(s));
        let two = |s: &[u8]| std::mem::transmute::<*mut c_void, Two>(need(s));
        let fma3 = find(b"_get_FMA3_enable\0");
        Crt {
            sin: one(b"sin\0"),
            cos: one(b"cos\0"),
            tan: one(b"tan\0"),
            asin: one(b"asin\0"),
            acos: one(b"acos\0"),
            atan: one(b"atan\0"),
            atan2: two(b"atan2\0"),
            exp: one(b"exp\0"),
            log: one(b"log\0"),
            log10: one(b"log10\0"),
            log2: one(b"log2\0"),
            pow: two(b"pow\0"),
            tanh: one(b"tanh\0"),
            sinh: one(b"sinh\0"),
            fma3: (!fma3.is_null()).then(|| std::mem::transmute::<*mut c_void, Flag>(fma3)),
        }
    }
}
